use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use crate::adapters::history::{
    ArchiveDocument, HistoryArchiveParser, ItemInput, category_counts, date_range, normalized_item,
};
use crate::domain::Platform;
use crate::domain::history::{
    ActivityDirection, ActivityItemKind, ActivityOwnership, HISTORY_SCHEMA_VERSION, HistoryPreview,
    HistoryWarning, NormalizedActivityItem, ParsedHistoryArchive,
};
use crate::error::{AppError, AppResult};

#[derive(Debug)]
pub struct LinkedInHistoryParser;

impl HistoryArchiveParser for LinkedInHistoryParser {
    fn platform(&self) -> Platform {
        Platform::Linkedin
    }

    fn parser_version(&self) -> &'static str {
        "linkedin-archive-v1"
    }

    fn probe(&self, document: &ArchiveDocument) -> u8 {
        let category_score = document
            .members
            .iter()
            .filter(|member| {
                matches!(
                    category_for_name(&member.name),
                    Some(
                        ActivityItemKind::Post
                            | ActivityItemKind::Comment
                            | ActivityItemKind::Message
                            | ActivityItemKind::Reaction
                            | ActivityItemKind::Connection
                    )
                )
            })
            .count()
            .checked_mul(20)
            .unwrap_or(100)
            .min(85) as u8;
        let linkedin_signature = document.members.iter().any(|member| {
            let name = member.name.to_ascii_lowercase();
            let sample = String::from_utf8_lossy(&member.bytes[..member.bytes.len().min(8_192)])
                .to_ascii_lowercase();
            name.contains("shares")
                || name.contains("connections")
                || name.contains("reactions")
                || sample.contains("sharecommentary")
                || (sample.contains("first name") && sample.contains("last name"))
                || sample.contains("conversation id")
        });
        if linkedin_signature {
            category_score.max(95)
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
                "LinkedIn archive contained no supported shares, comments, messages, reactions, or connections"
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
    if name.contains("share") || name.contains("post") {
        Some(ActivityItemKind::Post)
    } else if name.contains("comment") {
        Some(ActivityItemKind::Comment)
    } else if name.contains("message") {
        Some(ActivityItemKind::Message)
    } else if name.contains("reaction") {
        Some(ActivityItemKind::Reaction)
    } else if name.contains("connection") {
        Some(ActivityItemKind::Connection)
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
        .map_err(|_| AppError::Validation("LinkedIn CSV has invalid headers".to_owned()))?
        .iter()
        .map(|header| header.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    for (index, record) in reader.records().enumerate() {
        let record = match record {
            Ok(record) => record,
            Err(_) => {
                warnings.push(row_warning(member, index + 2));
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
                "sharecommentary",
                "comment",
                "content",
                "message",
                "body",
                "company",
                "position",
                "reactiontype",
            ],
        )
        .unwrap_or_default();
        let remote_id = field(&values, &["id", "conversation id", "message id"]);
        let canonical_url = field(
            &values,
            &["sharelink", "link", "url", "profile url", "profileurl"],
        );
        let published_at = field(
            &values,
            &[
                "date",
                "created at",
                "createdat",
                "timestamp",
                "connected on",
            ],
        );
        let author = field(&values, &["from", "author", "first name"]);
        let counterparty = field(&values, &["to", "recipient", "last name"]);
        let direction = (kind == ActivityItemKind::Message).then_some(ActivityDirection::Outbound);
        if body.is_empty() && canonical_url.is_none() && counterparty.is_none() {
            continue;
        }
        items.push(normalized_item(ItemInput {
            platform: Platform::Linkedin,
            item_kind: kind,
            ownership: if kind == ActivityItemKind::Connection {
                ActivityOwnership::Reference
            } else {
                ActivityOwnership::Own
            },
            direction,
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

fn row_warning(member: &str, row: usize) -> HistoryWarning {
    HistoryWarning {
        code: "invalid_row".to_owned(),
        message: "A malformed CSV row was skipped.".to_owned(),
        member: Some(member.to_owned()),
        row: Some(row as u64),
    }
}
