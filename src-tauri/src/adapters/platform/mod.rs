pub mod http;
pub mod linkedin;
pub mod oauth;
pub mod reddit;
pub mod x;

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::domain::{CapabilityState, Platform};
use crate::error::{AppError, AppResult};

use self::http::HttpTransport;
use self::linkedin::LinkedInAdapter;
use self::reddit::RedditAdapter;
use self::x::XAdapter;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub authenticate: CapabilityState,
    pub publish: CapabilityState,
    pub read_own_content: CapabilityState,
    pub metrics: CapabilityState,
    pub reply: CapabilityState,
    pub direct_messages: CapabilityState,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlatformRequestContext {
    pub access_token: String,
    pub account_id: String,
    pub display_name: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishRequest {
    pub body: String,
    pub title: Option<String>,
    pub destination: Option<String>,
    pub reply_to_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoteContent {
    pub platform: Platform,
    pub remote_id: String,
    pub body: String,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplyRequest {
    pub remote_parent_id: String,
    pub body: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectMessageRequest {
    pub recipient_id: String,
    pub body: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMessage {
    pub platform: Platform,
    pub remote_id: String,
    pub conversation_id: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncPage {
    pub items: Vec<RemoteContent>,
    pub next_cursor: Option<String>,
}

#[async_trait]
pub trait PlatformAdapter: Debug + Send + Sync {
    fn platform(&self) -> Platform;
    fn capabilities(&self, scopes: &[String]) -> PlatformCapabilities;
    async fn publish(
        &self,
        context: &PlatformRequestContext,
        request: PublishRequest,
    ) -> AppResult<RemoteContent>;
    async fn sync(
        &self,
        context: &PlatformRequestContext,
        cursor: Option<&str>,
    ) -> AppResult<SyncPage>;
    async fn reply(
        &self,
        context: &PlatformRequestContext,
        request: ReplyRequest,
    ) -> AppResult<RemoteMessage>;
    async fn send_direct_message(
        &self,
        context: &PlatformRequestContext,
        request: DirectMessageRequest,
    ) -> AppResult<RemoteMessage>;
}

#[derive(Debug, Clone)]
pub struct PlatformRegistry {
    x: Arc<XAdapter>,
    reddit: Arc<RedditAdapter>,
    linkedin: Arc<LinkedInAdapter>,
}

impl Default for PlatformRegistry {
    fn default() -> Self {
        let transport = HttpTransport::production();
        Self {
            x: Arc::new(XAdapter::new(transport.clone(), "https://api.x.com")),
            reddit: Arc::new(RedditAdapter::new(
                transport.clone(),
                "https://oauth.reddit.com",
            )),
            linkedin: Arc::new(LinkedInAdapter::new(
                transport,
                "https://api.linkedin.com",
                "202606",
            )),
        }
    }
}

impl PlatformRegistry {
    pub fn get(&self, platform: Platform) -> Arc<dyn PlatformAdapter> {
        match platform {
            Platform::X => self.x.clone(),
            Platform::Reddit => self.reddit.clone(),
            Platform::Linkedin => self.linkedin.clone(),
        }
    }
}

pub fn unsupported(platform: Platform, capability: &str, reason: &str) -> AppError {
    AppError::Unsupported(format!(
        "{} does not support {capability}: {reason}",
        platform.as_str()
    ))
}

#[cfg(test)]
mod tests {
    use crate::domain::{CapabilityState, Platform};

    use super::{PlatformRegistry, unsupported};

    #[test]
    fn registry_preserves_platform_identity() {
        let registry = PlatformRegistry::default();
        for platform in Platform::ALL {
            assert_eq!(registry.get(platform).platform(), platform);
        }
    }

    #[test]
    fn unsupported_is_a_typed_error() {
        let error = unsupported(Platform::Linkedin, "direct messages", "not public");
        assert!(matches!(error, crate::error::AppError::Unsupported(_)));
        let capabilities = PlatformRegistry::default()
            .get(Platform::Linkedin)
            .capabilities(&[]);
        assert_eq!(capabilities.direct_messages, CapabilityState::Unsupported);
    }
}
