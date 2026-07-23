use chrono::Utc;
use serde_json::Value;
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
pub struct XHistoryParser;

impl HistoryArchiveParser for XHistoryParser {
    fn platform(&self) -> Platform {
        Platform::X
    }

    fn parser_version(&self) -> &'static str {
        "x-archive-v1"
    }

    fn probe(&self, document: &ArchiveDocument) -> u8 {
        document
            .members
            .iter()
            .map(|member| {
                let name = member.name.to_ascii_lowercase();
                if name.contains("tweet")
                    || name.contains("direct-message")
                    || name.contains("account")
                    || member.bytes.windows(10).any(|value| value == b"window.YTD")
                {
                    90
                } else {
                    0
                }
            })
            .max()
            .unwrap_or_default()
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
        let mut account_handle = None;
        for member in &document.members {
            let name = member.name.to_ascii_lowercase();
            if !name.contains("tweet")
                && !name.contains("direct-message")
                && !name.contains("account")
                && !name.contains("profile")
            {
                continue;
            }
            let value = match parse_assignment_json(&member.bytes) {
                Ok(value) => value,
                Err(error) => {
                    warnings.push(HistoryWarning {
                        code: "invalid_member".to_owned(),
                        message: "A data member could not be parsed and was skipped.".to_owned(),
                        member: Some(member.name.clone()),
                        row: None,
                    });
                    tracing::debug!(member = %member.name, error = %error, "X archive member skipped");
                    continue;
                }
            };
            walk_values(
                &value,
                &member.name,
                &observed_at,
                &mut account_handle,
                &mut items,
            );
        }
        if items.is_empty() {
            return Err(AppError::Validation(
                "X archive contained no supported posts, messages, or profile records".to_owned(),
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
                account_handle,
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

fn parse_assignment_json(bytes: &[u8]) -> AppResult<Value> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AppError::Validation("archive member is not valid UTF-8".to_owned()))?
        .trim()
        .trim_start_matches('\u{feff}');
    let start = text
        .find(['[', '{'])
        .ok_or_else(|| AppError::Validation("archive member has no JSON value".to_owned()))?;
    let value = text[start..].trim().trim_end_matches(';').trim();
    Ok(serde_json::from_str(value)?)
}

fn walk_values(
    value: &Value,
    member: &str,
    observed_at: &str,
    account_handle: &mut Option<String>,
    items: &mut Vec<NormalizedActivityItem>,
) {
    match value {
        Value::Array(values) => {
            for value in values {
                walk_values(value, member, observed_at, account_handle, items);
            }
        }
        Value::Object(object) => {
            if let Some(tweet) = object.get("tweet").and_then(Value::as_object) {
                let body = string_field(tweet, &["full_text", "fullText", "text"]);
                if let Some(body) = body {
                    let remote_id = string_field(tweet, &["id_str", "idStr", "id"]);
                    let published_at = string_field(tweet, &["created_at", "createdAt"]);
                    let item_kind = if tweet
                        .get("in_reply_to_status_id")
                        .or_else(|| tweet.get("inReplyToStatusId"))
                        .is_some_and(|value| !value.is_null())
                    {
                        ActivityItemKind::Reply
                    } else {
                        ActivityItemKind::Post
                    };
                    let canonical = account_handle.as_deref().and_then(|handle| {
                        remote_id
                            .as_deref()
                            .map(|id| format!("https://x.com/{handle}/status/{id}"))
                    });
                    items.push(normalized_item(ItemInput {
                        platform: Platform::X,
                        item_kind,
                        ownership: ActivityOwnership::Own,
                        direction: None,
                        remote_id: remote_id.as_deref(),
                        canonical_url: canonical.as_deref(),
                        author_handle: account_handle.as_deref(),
                        counterparty_handle: None,
                        body: &body,
                        published_at: published_at.as_deref(),
                        observed_at,
                        metadata: serde_json::json!({"member": member}),
                    }));
                }
            }
            if let Some(message) = object
                .get("messageCreate")
                .or_else(|| object.get("message"))
                .and_then(Value::as_object)
                && let Some(body) = string_field(message, &["text", "body"])
            {
                let remote_id = string_field(message, &["id", "id_str", "idStr"]);
                let published_at = string_field(message, &["createdAt", "created_at", "created"]);
                items.push(normalized_item(ItemInput {
                    platform: Platform::X,
                    item_kind: ActivityItemKind::Message,
                    ownership: ActivityOwnership::Own,
                    direction: None,
                    remote_id: remote_id.as_deref(),
                    canonical_url: None,
                    author_handle: None,
                    counterparty_handle: None,
                    body: &body,
                    published_at: published_at.as_deref(),
                    observed_at,
                    metadata: serde_json::json!({"member": member, "readOnly": true}),
                }));
            }
            if let Some(account) = object
                .get("account")
                .or_else(|| object.get("profile"))
                .and_then(Value::as_object)
            {
                if account_handle.is_none() {
                    *account_handle =
                        string_field(account, &["username", "screen_name", "screenName"]);
                }
                if let Some(body) = string_field(account, &["bio", "description"]) {
                    let remote_id = string_field(account, &["accountId", "id"]);
                    items.push(normalized_item(ItemInput {
                        platform: Platform::X,
                        item_kind: ActivityItemKind::Profile,
                        ownership: ActivityOwnership::Own,
                        direction: None,
                        remote_id: remote_id.as_deref(),
                        canonical_url: None,
                        author_handle: account_handle.as_deref(),
                        counterparty_handle: None,
                        body: &body,
                        published_at: None,
                        observed_at,
                        metadata: serde_json::json!({"member": member}),
                    }));
                }
            }
            for child in object.values() {
                if child.is_array() || child.is_object() {
                    walk_values(child, member, observed_at, account_handle, items);
                }
            }
        }
        _ => {}
    }
}

fn string_field(object: &serde_json::Map<String, Value>, candidates: &[&str]) -> Option<String> {
    candidates.iter().find_map(|candidate| {
        object.get(*candidate).and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::parse_assignment_json;

    #[test]
    fn parses_assignment_wrapped_json_as_inert_data() {
        let value = parse_assignment_json(
            br#"window.YTD.tweets.part0 = [{"tweet":{"id":"1","full_text":"hello"}}];"#,
        )
        .expect("JSON");
        assert_eq!(value[0]["tweet"]["full_text"], "hello");
    }
}
