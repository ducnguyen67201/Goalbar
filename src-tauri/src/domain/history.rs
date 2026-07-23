use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::Platform;

pub const HISTORY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HistorySourceKind {
    Archive,
    Browser,
}

impl HistorySourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Archive => "archive",
            Self::Browser => "browser",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityOwnership {
    Own,
    Reference,
}

impl ActivityOwnership {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::Reference => "reference",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityItemKind {
    Post,
    Comment,
    Reply,
    Message,
    Reaction,
    Connection,
    Profile,
}

impl ActivityItemKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Post => "post",
            Self::Comment => "comment",
            Self::Reply => "reply",
            Self::Message => "message",
            Self::Reaction => "reaction",
            Self::Connection => "connection",
            Self::Profile => "profile",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityDirection {
    Inbound,
    Outbound,
}

impl ActivityDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistorySelection {
    pub selection_id: Uuid,
    pub display_name: String,
    pub size_bytes: u64,
    pub container: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCategoryCount {
    pub category: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryWarning {
    pub code: String,
    pub message: String,
    pub member: Option<String>,
    pub row: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPreview {
    pub schema_version: u32,
    pub selection_id: Uuid,
    pub platform: Platform,
    pub parser_version: String,
    pub display_name: String,
    pub account_handle: Option<String>,
    pub categories: Vec<HistoryCategoryCount>,
    pub estimated_records: u32,
    pub earliest_at: Option<String>,
    pub latest_at: Option<String>,
    pub warnings: Vec<HistoryWarning>,
    pub unsupported_members: Vec<String>,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryImportResult {
    pub source_id: Uuid,
    pub run_id: Uuid,
    pub platform: Platform,
    pub imported: u32,
    pub skipped: u32,
    pub warning_count: u32,
    pub duplicate_source: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryOverviewPlatform {
    pub platform: Platform,
    pub source_count: u32,
    pub item_count: u32,
    pub own_item_count: u32,
    pub reference_item_count: u32,
    pub latest_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryOverview {
    pub schema_version: u32,
    pub source_count: u32,
    pub item_count: u32,
    pub platforms: Vec<HistoryOverviewPlatform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedActivityItem {
    pub platform: Platform,
    pub item_kind: ActivityItemKind,
    pub ownership: ActivityOwnership,
    pub direction: Option<ActivityDirection>,
    pub remote_id: Option<String>,
    pub canonical_url: Option<String>,
    pub author_handle: Option<String>,
    pub counterparty_handle: Option<String>,
    pub body: String,
    pub published_at: Option<String>,
    pub observed_at: String,
    pub dedupe_key: String,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
pub struct ParsedHistoryArchive {
    pub preview: HistoryPreview,
    pub items: Vec<NormalizedActivityItem>,
}
