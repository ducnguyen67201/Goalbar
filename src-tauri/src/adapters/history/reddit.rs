use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use crate::adapters::history::{
    ArchiveDocument, HistoryArchiveParser, ItemInput, category_counts, date_range, normalized_item,
};
use crate::domain::Platform;
use crate::domain::history::{
    ActivityItemKind, ActivityOwnership, HISTORY_SCHEMA_VERSION, HistoryPreview, HistoryWarning,
    NormalizedActivityItem, ParsedHistoryArchive,
};
use crate::error::{AppError, AppResult};

#[derive(Debug)]
pub struct RedditHistoryParser;

impl HistoryArchiveParser for RedditHistoryParser {
    fn platform(&self) -> Platform {
        Platform::Reddit
    }

    fn parser_version(&self) -> &'static str {
        "reddit-archive-v1"
    }

    fn probe(&self, document: &ArchiveDocument) -> u8 {
        let category_score = document
            .members
            .iter()
            .filter(|member| category_for_name(&member.name).is_some())
            .count()
            .checked_mul(20)
            .unwrap_or(100)
            .min(80) as u8;
        let reddit_signature = document.members.iter().any(|member| {
            let sample = String::from_utf8_lossy(&member.bytes[..member.bytes.len().min(8_192)])
                .to_ascii_lowercase();
            sample.contains("subreddit")
                || sample.contains("created_utc")
                || sample.contains("permalink")
                || sample.contains("vote_direction")
        });
        if reddit_signature {
            category_score.max(90)
        } else {
            category_score
        }
    }

    fn parse(
        &self,
        document: &ArchiveDocument,
        selection_id: Uuid,
        display_name: &str,
        fingerprint: &str,
    ) -> AppResult<ParsedHistoryArchive> {
        let observed_at = Utc::now().to_rfc3339();
        let mut items = Vec::new();
        let mut warnings = Vec::new();
        for member in &document.members {
            let Some(kind) = category_for_name(&member.name) else {
                continue;
            };
            parse_csv_member(
                &member.name,
                &member.bytes,
                kind,
                &observed_at,
                &mut items,
                &mut warnings,
            )?;
        }
        if items.is_empty() {
            return Err(AppError::Validation(
                "Reddit archive contained no supported posts, comments, messages, or votes"
                    .to_owned(),
            ));
        }
        let (earliest_at, latest_at) = date_range(&items);
        Ok(ParsedHistoryArchive {
            preview: HistoryPreview {
                schema_version: HISTORY_SCHEMA_VERSION,
                selection_id,
                platform: self.platform(),
                parser_version: self.parser_version().to_owned(),
                display_name: display_name.to_owned(),
                account_handle: None,
                categories: category_counts(&items),
                estimated_records: items.len() as u32,
                earliest_at,
                latest_at,
                warnings,
                unsupported_members: document.unsupported_members.clone(),
                source_fingerprint: fingerprint.to_owned(),
            },
            items,
        })
    }
}

fn category_for_name(name: &str) -> Option<ActivityItemKind> {
    let name = name.to_ascii_lowercase();
    if name.contains("post") {
        Some(ActivityItemKind::Post)
    } else if name.contains("comment") {
        Some(ActivityItemKind::Comment)
    } else if name.contains("message") || name.contains("chat") {
        Some(ActivityItemKind::Message)
    } else if name.contains("vote") {
        Some(ActivityItemKind::Reaction)
    } else {
        None
    }
}

fn parse_csv_member(
    member: &str,
    bytes: &[u8],
    kind: ActivityItemKind,
    observed_at: &str,
    items: &mut Vec<NormalizedActivityItem>,
    warnings: &mut Vec<HistoryWarning>,
) -> AppResult<()> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|_| AppError::Validation("Reddit CSV has invalid headers".to_owned()))?
        .iter()
        .map(|header| header.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    for (index, record) in reader.records().enumerate() {
        let record = match record {
            Ok(record) => record,
            Err(_) => {
                warnings.push(HistoryWarning {
                    code: "invalid_row".to_owned(),
                    message: "A malformed CSV row was skipped.".to_owned(),
                    member: Some(member.to_owned()),
                    row: Some((index + 2) as u64),
                });
                continue;
            }
        };
        let values = headers
            .iter()
            .zip(record.iter())
            .map(|(header, value)| (header.as_str(), value))
            .collect::<HashMap<_, _>>();
        let body = field(
            &values,
            &[
                "body",
                "selftext",
                "title",
                "message",
                "content",
                "vote_direction",
            ],
        )
        .unwrap_or_default();
        let remote_id = field(&values, &["id", "post_id", "comment_id", "message_id"]);
        let canonical_url = field(&values, &["permalink", "url", "link"]);
        let published_at = field(&values, &["date", "created_utc", "created at", "timestamp"]);
        let author = field(&values, &["author", "username", "from"]);
        let counterparty = field(&values, &["to", "recipient", "subreddit"]);
        if body.is_empty() && canonical_url.is_none() {
            continue;
        }
        items.push(normalized_item(ItemInput {
            platform: Platform::Reddit,
            item_kind: kind,
            ownership: ActivityOwnership::Own,
            direction: None,
            remote_id,
            canonical_url,
            author_handle: author,
            counterparty_handle: counterparty,
            body,
            published_at,
            observed_at,
            metadata: serde_json::json!({"member": member, "readOnly": kind == ActivityItemKind::Message}),
        }));
    }
    Ok(())
}

fn field<'a>(values: &HashMap<&str, &'a str>, candidates: &[&str]) -> Option<&'a str> {
    candidates
        .iter()
        .find_map(|candidate| values.get(candidate).copied())
        .filter(|value| !value.trim().is_empty())
}
