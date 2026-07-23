use async_trait::async_trait;
use reqwest::Method;

use super::http::HttpTransport;
use super::{
    DirectMessageRequest, PlatformAdapter, PlatformCapabilities, PlatformRequestContext,
    PublishRequest, RemoteContent, RemoteMessage, ReplyRequest, SyncPage, unsupported,
};
use crate::domain::{CapabilityState, Platform};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct LinkedInAdapter {
    transport: HttpTransport,
    base_url: String,
    api_version: String,
}

impl LinkedInAdapter {
    pub fn new(
        transport: HttpTransport,
        base_url: impl Into<String>,
        api_version: impl Into<String>,
    ) -> Self {
        Self {
            transport,
            base_url: base_url.into(),
            api_version: api_version.into(),
        }
    }

    fn headers(&self) -> [(&str, &str); 2] {
        [
            ("Linkedin-Version", self.api_version.as_str()),
            ("X-Restli-Protocol-Version", "2.0.0"),
        ]
    }
}

#[async_trait]
impl PlatformAdapter for LinkedInAdapter {
    fn platform(&self) -> Platform {
        Platform::Linkedin
    }

    fn capabilities(&self, scopes: &[String]) -> PlatformCapabilities {
        let has = |scope: &str| scopes.iter().any(|value| value == scope);
        PlatformCapabilities {
            authenticate: CapabilityState::ApprovalPending,
            publish: if has("w_member_social") { CapabilityState::Supported } else { CapabilityState::ApprovalPending },
            read_own_content: if has("r_member_social") { CapabilityState::Supported } else { CapabilityState::ApprovalPending },
            metrics: if has("r_member_social") { CapabilityState::Supported } else { CapabilityState::ApprovalPending },
            reply: if has("w_member_social") && has("r_member_social") { CapabilityState::Supported } else { CapabilityState::ApprovalPending },
            direct_messages: CapabilityState::Unsupported,
            detail: Some("General member DMs are not a public self-service capability. Open the conversation in LinkedIn.".to_owned()),
        }
    }

    async fn publish(
        &self,
        context: &PlatformRequestContext,
        request: PublishRequest,
    ) -> AppResult<RemoteContent> {
        if !context.account_id.starts_with("urn:li:") {
            return Err(AppError::Validation(
                "LinkedIn account ID must be an actor URN".to_owned(),
            ));
        }
        let response = self
            .transport
            .json(
                Method::POST,
                &format!("{}/rest/posts", self.base_url),
                &context.access_token,
                &self.headers(),
                Some(&serde_json::json!({
                    "author": context.account_id,
                    "commentary": request.body,
                    "visibility": "PUBLIC",
                    "distribution": { "feedDistribution": "MAIN_FEED", "targetEntities": [], "thirdPartyDistributionChannels": [] },
                    "lifecycleState": "PUBLISHED",
                    "isReshareDisabledByAuthor": false
                })),
            )
            .await?;
        let id = response
            .headers
            .get("x-restli-id")
            .cloned()
            .or_else(|| {
                response
                    .body
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| {
                AppError::Platform("LinkedIn response did not include a post id".to_owned())
            })?;
        Ok(RemoteContent {
            platform: Platform::Linkedin,
            remote_id: id.clone(),
            body: request.body,
            remote_url: Some(format!("https://www.linkedin.com/feed/update/{id}")),
        })
    }

    async fn sync(
        &self,
        context: &PlatformRequestContext,
        cursor: Option<&str>,
    ) -> AppResult<SyncPage> {
        if !context
            .scopes
            .iter()
            .any(|scope| scope == "r_member_social")
        {
            return Err(unsupported(
                Platform::Linkedin,
                "read member activity",
                "r_member_social is restricted and not granted",
            ));
        }
        let start = cursor.unwrap_or("0");
        let encoded =
            url::form_urlencoded::byte_serialize(context.account_id.as_bytes()).collect::<String>();
        let url = format!(
            "{}/rest/posts?author={encoded}&q=author&count=25&start={start}",
            self.base_url
        );
        let response = self
            .transport
            .json(
                Method::GET,
                &url,
                &context.access_token,
                &self.headers(),
                None,
            )
            .await?;
        let items = response
            .body
            .get("elements")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| {
                let id = item.get("id")?.as_str()?.to_owned();
                Some(RemoteContent {
                    platform: Platform::Linkedin,
                    remote_url: Some(format!("https://www.linkedin.com/feed/update/{id}")),
                    remote_id: id,
                    body: item
                        .get("commentary")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                })
            })
            .collect();
        Ok(SyncPage {
            items,
            next_cursor: None,
        })
    }

    async fn reply(
        &self,
        context: &PlatformRequestContext,
        request: ReplyRequest,
    ) -> AppResult<RemoteMessage> {
        if !context
            .scopes
            .iter()
            .any(|scope| scope == "r_member_social")
        {
            return Err(unsupported(
                Platform::Linkedin,
                "comments",
                "Community Management read access is not granted",
            ));
        }
        let encoded = url::form_urlencoded::byte_serialize(request.remote_parent_id.as_bytes())
            .collect::<String>();
        let response = self
            .transport
            .json(
                Method::POST,
                &format!("{}/rest/socialActions/{encoded}/comments", self.base_url),
                &context.access_token,
                &self.headers(),
                Some(&serde_json::json!({ "actor": context.account_id, "message": { "text": request.body } })),
            )
            .await?;
        let id = response
            .headers
            .get("x-restli-id")
            .cloned()
            .unwrap_or(request.idempotency_key);
        Ok(RemoteMessage {
            platform: Platform::Linkedin,
            remote_id: id,
            conversation_id: Some(request.remote_parent_id),
            body: request.body,
        })
    }

    async fn send_direct_message(
        &self,
        _context: &PlatformRequestContext,
        _request: DirectMessageRequest,
    ) -> AppResult<RemoteMessage> {
        Err(unsupported(
            Platform::Linkedin,
            "general member direct messages",
            "use the Open in LinkedIn action",
        ))
    }
}
