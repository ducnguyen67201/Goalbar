use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{CapabilityState, Platform};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipIdentity {
    pub id: Uuid,
    pub platform: Platform,
    pub remote_id: String,
    pub kind: String,
    pub handle: String,
    pub profile_url: Option<String>,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: Uuid,
    pub platform: Platform,
    pub remote_id: String,
    pub kind: String,
    pub display_name: String,
    pub preview: String,
    pub unread_count: u32,
    pub reply_capability: CapabilityState,
    pub remote_url: Option<String>,
    pub source: ConversationSource,
    pub content_state: ConversationContentState,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConversationSource {
    PlatformApi,
    EmailNotification,
    BrowserScan,
}

impl ConversationSource {
    pub fn parse(value: &str) -> crate::error::AppResult<Self> {
        match value {
            "platform_api" => Ok(Self::PlatformApi),
            "email_notification" => Ok(Self::EmailNotification),
            "browser_scan" => Ok(Self::BrowserScan),
            _ => Err(crate::error::AppError::Validation(format!(
                "unknown conversation source: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserInboxScanStatus {
    Completed,
    NeedsBrowser,
    LoginRequired,
    VerificationRequired,
    UnsupportedPage,
}

impl BrowserInboxScanStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::NeedsBrowser => "needs_browser",
            Self::LoginRequired => "login_required",
            Self::VerificationRequired => "verification_required",
            Self::UnsupportedPage => "unsupported_page",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserInboxScanResult {
    pub platform: Platform,
    pub status: BrowserInboxScanStatus,
    pub scanned: u32,
    pub imported: u32,
    pub updated: u32,
    pub last_scanned_at: String,
    pub message: String,
    pub target_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConversationContentState {
    Complete,
    NotificationExcerpt,
    LinkOnly,
}

impl ConversationContentState {
    pub fn parse(value: &str) -> crate::error::AppResult<Self> {
        match value {
            "complete" => Ok(Self::Complete),
            "notification_excerpt" => Ok(Self::NotificationExcerpt),
            "link_only" => Ok(Self::LinkOnly),
            _ => Err(crate::error::AppError::Validation(format!(
                "unknown conversation content state: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmailNotificationPlatformCounts {
    pub x: u32,
    pub reddit: u32,
    pub linkedin: u32,
}

impl EmailNotificationPlatformCounts {
    pub const fn empty() -> Self {
        Self {
            x: 0,
            reddit: 0,
            linkedin: 0,
        }
    }

    pub fn increment(&mut self, platform: Platform) {
        match platform {
            Platform::X => self.x += 1,
            Platform::Reddit => self.reddit += 1,
            Platform::Linkedin => self.linkedin += 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmailNotificationSyncResult {
    pub source: String,
    pub scanned: u32,
    pub imported: u32,
    pub ignored: u32,
    pub duplicates: u32,
    pub platform_counts: EmailNotificationPlatformCounts,
    pub last_checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplyOptions {
    pub options: Vec<String>,
}
