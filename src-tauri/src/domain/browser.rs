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
