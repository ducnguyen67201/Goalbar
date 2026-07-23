pub mod linkedin;
pub mod reddit;
pub mod x;

use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Take};
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;
use zip::ZipArchive;

use crate::domain::Platform;
use crate::domain::history::{
    ActivityDirection, ActivityItemKind, ActivityOwnership, NormalizedActivityItem,
    ParsedHistoryArchive,
};
use crate::error::{AppError, AppResult};

use self::linkedin::LinkedInHistoryParser;
use self::reddit::RedditHistoryParser;
use self::x::XHistoryParser;

pub const MAX_ARCHIVE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_MEMBER_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ArchiveMember {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ArchiveDocument {
    pub members: Vec<ArchiveMember>,
    pub unsupported_members: Vec<String>,
}

impl ArchiveDocument {
    pub fn load(path: &Path) -> AppResult<Self> {
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > MAX_ARCHIVE_BYTES {
            return Err(AppError::Validation(
                "archive exceeds the 1 GiB safety limit".to_owned(),
            ));
        }
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("zip"))
        {
            return Self::load_zip(path);
        }
        let file = File::open(path)?;
        let bytes = read_limited(file.take(MAX_MEMBER_BYTES + 1), MAX_MEMBER_BYTES)?;
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("archive-data")
            .to_owned();
        Ok(Self {
            members: vec![ArchiveMember { name, bytes }],
            unsupported_members: Vec::new(),
        })
    }

    fn load_zip(path: &Path) -> AppResult<Self> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)
            .map_err(|error| AppError::Validation(format!("invalid ZIP archive: {error}")))?;
        if archive.len() > MAX_ARCHIVE_ENTRIES {
            return Err(AppError::Validation(format!(
                "archive contains more than {MAX_ARCHIVE_ENTRIES} entries"
            )));
        }
        let mut members = Vec::new();
        let mut unsupported_members = Vec::new();
        let mut total_bytes = 0_u64;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| AppError::Validation(format!("invalid ZIP member: {error}")))?;
            if entry.is_dir() {
                continue;
            }
            let Some(enclosed) = entry.enclosed_name() else {
                return Err(AppError::Validation(
                    "archive contains an unsafe member path".to_owned(),
                ));
            };
            let name = enclosed.to_string_lossy().into_owned();
            if entry.size() > MAX_MEMBER_BYTES {
                return Err(AppError::Validation(format!(
                    "archive data member exceeds the 128 MiB safety limit: {name}"
                )));
            }
            total_bytes = total_bytes.saturating_add(entry.size());
            if total_bytes > MAX_TOTAL_BYTES {
                return Err(AppError::Validation(
                    "archive expanded data exceeds the 2 GiB safety limit".to_owned(),
                ));
            }
            if is_supported_text_member(&enclosed) {
                let bytes =
                    read_limited((&mut entry).take(MAX_MEMBER_BYTES + 1), MAX_MEMBER_BYTES)?;
                members.push(ArchiveMember { name, bytes });
            } else if unsupported_members.len() < 100 && !is_media_member(&enclosed) {
                unsupported_members.push(name);
            }
        }
        if members.is_empty() {
            return Err(AppError::Validation(
                "archive contains no supported CSV, JSON, or JavaScript data files".to_owned(),
            ));
        }
        Ok(Self {
            members,
            unsupported_members,
        })
    }
}

pub trait HistoryArchiveParser: Debug + Send + Sync {
    fn platform(&self) -> Platform;
    fn parser_version(&self) -> &'static str;
    fn probe(&self, document: &ArchiveDocument) -> u8;
    fn parse(
        &self,
        document: &ArchiveDocument,
        selection_id: Uuid,
        display_name: &str,
        fingerprint: &str,
    ) -> AppResult<ParsedHistoryArchive>;
}

#[derive(Debug, Clone)]
pub struct HistoryParserRegistry {
    parsers: Vec<Arc<dyn HistoryArchiveParser>>,
}

impl Default for HistoryParserRegistry {
    fn default() -> Self {
        Self {
            parsers: vec![
                Arc::new(XHistoryParser),
                Arc::new(LinkedInHistoryParser),
                Arc::new(RedditHistoryParser),
            ],
        }
    }
}

impl HistoryParserRegistry {
    pub fn parse(
        &self,
        path: &Path,
        selection_id: Uuid,
        display_name: &str,
        fingerprint: &str,
    ) -> AppResult<ParsedHistoryArchive> {
        let document = ArchiveDocument::load(path)?;
        let parser = self
            .parsers
            .iter()
            .max_by_key(|parser| parser.probe(&document))
            .filter(|parser| parser.probe(&document) > 0)
            .ok_or_else(|| {
                AppError::Unsupported(
                    "archive did not match a supported X, LinkedIn, or Reddit export".to_owned(),
                )
            })?;
        parser.parse(&document, selection_id, display_name, fingerprint)
    }
}

pub fn file_fingerprint(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) struct ItemInput<'a> {
    pub platform: Platform,
    pub item_kind: ActivityItemKind,
    pub ownership: ActivityOwnership,
    pub direction: Option<ActivityDirection>,
    pub remote_id: Option<&'a str>,
    pub canonical_url: Option<&'a str>,
    pub author_handle: Option<&'a str>,
    pub counterparty_handle: Option<&'a str>,
    pub body: &'a str,
    pub published_at: Option<&'a str>,
    pub observed_at: &'a str,
    pub metadata: serde_json::Value,
}

pub(crate) fn normalized_item(input: ItemInput<'_>) -> NormalizedActivityItem {
    let body = normalize_text(input.body, 100_000);
    let identity = format!(
        "{}\n{}\n{}\n{}\n{}",
        input.platform.as_str(),
        input.item_kind.as_str(),
        input.remote_id.unwrap_or_default(),
        input.canonical_url.unwrap_or_default(),
        body
    );
    NormalizedActivityItem {
        platform: input.platform,
        item_kind: input.item_kind,
        ownership: input.ownership,
        direction: input.direction,
        remote_id: input.remote_id.map(str::to_owned),
        canonical_url: input.canonical_url.map(str::to_owned),
        author_handle: input.author_handle.map(str::to_owned),
        counterparty_handle: input.counterparty_handle.map(str::to_owned),
        body,
        published_at: input.published_at.and_then(normalize_timestamp),
        observed_at: input.observed_at.to_owned(),
        dedupe_key: format!("{:x}", Sha256::digest(identity.as_bytes())),
        metadata: input.metadata,
    }
}

fn normalize_timestamp(value: &str) -> Option<String> {
    let value = value.trim();
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return Some(timestamp.with_timezone(&Utc).to_rfc3339());
    }
    if let Ok(timestamp) = DateTime::parse_from_str(value, "%a %b %d %H:%M:%S %z %Y") {
        return Some(timestamp.with_timezone(&Utc).to_rfc3339());
    }
    if let Ok(timestamp) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc).to_rfc3339());
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return date.and_hms_opt(0, 0, 0).map(|timestamp| {
            DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc).to_rfc3339()
        });
    }
    value.parse::<i64>().ok().and_then(|number| {
        let (seconds, nanos) = if number.unsigned_abs() > 10_000_000_000 {
            (
                number.div_euclid(1_000),
                (number.rem_euclid(1_000) as u32) * 1_000_000,
            )
        } else {
            (number, 0)
        };
        DateTime::<Utc>::from_timestamp(seconds, nanos).map(|timestamp| timestamp.to_rfc3339())
    })
}

pub(crate) fn normalize_text(value: &str, max_chars: usize) -> String {
    value
        .trim_start_matches('\u{feff}')
        .chars()
        .map(|character| {
            if character.is_control() && !matches!(character, '\n' | '\t') {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

pub(crate) fn date_range(items: &[NormalizedActivityItem]) -> (Option<String>, Option<String>) {
    let mut values = items
        .iter()
        .filter_map(|item| item.published_at.clone())
        .collect::<Vec<_>>();
    values.sort();
    (values.first().cloned(), values.last().cloned())
}

pub(crate) fn category_counts(
    items: &[NormalizedActivityItem],
) -> Vec<crate::domain::history::HistoryCategoryCount> {
    let mut counts = std::collections::BTreeMap::<String, u32>::new();
    for item in items {
        *counts
            .entry(item.item_kind.as_str().to_owned())
            .or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(category, count)| crate::domain::history::HistoryCategoryCount { category, count })
        .collect()
}

fn read_limited(mut reader: Take<impl Read>, limit: u64) -> AppResult<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(AppError::Validation(
            "archive data member exceeds its safety limit".to_owned(),
        ));
    }
    Ok(bytes)
}

fn is_supported_text_member(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "csv" | "json" | "js" | "txt"
            )
        })
}

fn is_media_member(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "webp" | "mp4" | "mov" | "mp3" | "wav" | "pdf"
            )
        })
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::{ArchiveDocument, normalize_text, normalize_timestamp};

    #[test]
    fn archive_text_is_inert_and_bounded() {
        assert_eq!(normalize_text("\u{feff}=cmd()\n hello", 20), "=cmd() hello");
        assert_eq!(normalize_text("abcdef", 3), "abc");
    }

    #[test]
    fn archive_timestamps_are_normalized_to_rfc3339() {
        assert_eq!(
            normalize_timestamp("Mon Feb 03 12:00:00 +0000 2025").as_deref(),
            Some("2025-02-03T12:00:00+00:00")
        );
        assert_eq!(
            normalize_timestamp("2025-02-03").as_deref(),
            Some("2025-02-03T00:00:00+00:00")
        );
    }

    #[test]
    fn archive_rejects_path_traversal_members() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("unsafe.zip");
        let file = std::fs::File::create(&path).expect("ZIP file");
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file(
                "../../comments.csv",
                zip::write::SimpleFileOptions::default(),
            )
            .expect("member");
        writer.write_all(b"id,body\n1,unsafe").expect("fixture");
        writer.finish().expect("finish ZIP");
        assert!(ArchiveDocument::load(&path).is_err());
    }
}
