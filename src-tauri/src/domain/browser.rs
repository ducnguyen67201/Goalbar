use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::Platform;

pub const BROWSER_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserLoadState {
    Idle,
    Loading,
    Loaded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserPageKind {
    Feed,
    Profile,
    Post,
    Messages,
    Search,
    Login,
    Challenge,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserPolicyState {
    ExplicitCapture,
    BoundedCollection,
    ManualOnly,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserRunStatus {
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserPauseReason {
    LoginRequired,
    VerificationRequired,
    RateLimited,
    UnsupportedPage,
    HostChanged,
    PolicyRestricted,
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserReplyPreparationStatus {
    Prepared,
    ComposerNotFound,
    LoginRequired,
    VerificationRequired,
    UnsupportedPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SavedBrowserReplyStatus {
    Prepared,
    ConfirmedPosted,
}

impl SavedBrowserReplyStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::ConfirmedPosted => "confirmed_posted",
        }
    }

    pub fn parse(value: &str) -> crate::error::AppResult<Self> {
        match value {
            "prepared" => Ok(Self::Prepared),
            "confirmed_posted" => Ok(Self::ConfirmedPosted),
            _ => Err(crate::error::AppError::Internal(format!(
                "unknown saved browser reply status: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserTab {
    pub id: Uuid,
    pub webview_label: String,
    pub current_url: String,
    pub title: String,
    pub load_state: BrowserLoadState,
    pub platform: Option<Platform>,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserReplyPreparation {
    pub status: BrowserReplyPreparationStatus,
    pub platform: Option<Platform>,
    pub character_count: u32,
    pub saved_reply: Option<SavedBrowserReply>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavedBrowserReply {
    pub id: Uuid,
    pub platform: Platform,
    pub target_url: String,
    pub exact_reply: String,
    pub status: SavedBrowserReplyStatus,
    pub prepared_at: String,
    pub confirmed_posted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserObservationBlock {
    pub key: String,
    pub role: String,
    pub text: String,
    pub links: Vec<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserViewport {
    pub width: u32,
    pub height: u32,
    pub scroll_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserObservation {
    pub schema_version: u32,
    pub tab_id: Uuid,
    pub url: String,
    pub title: String,
    pub platform: Option<Platform>,
    pub page_kind: BrowserPageKind,
    pub viewport: BrowserViewport,
    pub visible_blocks: Vec<BrowserObservationBlock>,
    pub captured_item_keys: Vec<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCapturePreview {
    pub observation: BrowserObservation,
    pub selected_text: Option<String>,
    pub normalized_item_count: u32,
    pub policy_state: BrowserPolicyState,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserRunLimits {
    pub maximum_items: u32,
    pub maximum_steps: u32,
    pub earliest_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRunProgress {
    pub run_id: Uuid,
    pub status: BrowserRunStatus,
    pub step: u32,
    pub item_count: u32,
    pub new_item_count: u32,
    pub pause_reason: Option<BrowserPauseReason>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BrowserAction {
    Observe,
    Navigate { url: String },
    Scroll { delta_y: i32 },
    CaptureVisible { ownership: String },
    CaptureSelection { ownership: String },
    RequestUserAction { reason: String, recovery: String },
    Stop { summary: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResearchFindingCategory {
    Pain,
    Goal,
    Objection,
    Language,
    Trigger,
    ContentTheme,
    CounterEvidence,
}

impl ResearchFindingCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pain => "pain",
            Self::Goal => "goal",
            Self::Objection => "objection",
            Self::Language => "language",
            Self::Trigger => "trigger",
            Self::ContentTheme => "content_theme",
            Self::CounterEvidence => "counter_evidence",
        }
    }

    pub fn parse(value: &str) -> crate::error::AppResult<Self> {
        match value {
            "pain" => Ok(Self::Pain),
            "goal" => Ok(Self::Goal),
            "objection" => Ok(Self::Objection),
            "language" => Ok(Self::Language),
            "trigger" => Ok(Self::Trigger),
            "content_theme" => Ok(Self::ContentTheme),
            "counter_evidence" => Ok(Self::CounterEvidence),
            _ => Err(crate::error::AppError::Internal(format!(
                "unknown research finding category: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResearchFindingStatus {
    Proposed,
    Accepted,
    Rejected,
}

impl ResearchFindingStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }

    pub fn parse(value: &str) -> crate::error::AppResult<Self> {
        match value {
            "proposed" => Ok(Self::Proposed),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            _ => Err(crate::error::AppError::Internal(format!(
                "unknown research finding status: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchFindingDraft {
    pub category: ResearchFindingCategory,
    pub summary: String,
    pub evidence_excerpt: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrowserResearchAction {
    Scroll { delta_y: i32 },
    OpenLink { url: String },
    GoBack,
    RequestUserAction { reason: String, recovery: String },
    Finish { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserResearchDecision {
    pub summary: String,
    pub findings: Vec<ResearchFindingDraft>,
    pub action: BrowserResearchAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StoredResearchFinding {
    pub id: Uuid,
    pub run_id: Uuid,
    pub platform: Platform,
    pub category: ResearchFindingCategory,
    pub summary: String,
    pub evidence_excerpt: String,
    pub source_url: String,
    pub confidence: f64,
    pub status: ResearchFindingStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserResearchTrace {
    pub id: Uuid,
    pub run_id: Uuid,
    pub step: u32,
    pub action: String,
    pub message: String,
    pub url: String,
    pub created_at: String,
}
