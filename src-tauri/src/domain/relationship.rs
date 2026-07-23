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
    pub display_name: String,
    pub preview: String,
    pub unread_count: u32,
    pub reply_capability: CapabilityState,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplyOptions {
    pub options: Vec<String>,
}
