use async_trait::async_trait;
use reqwest::Method;

use super::{
    DirectMessageRequest, PlatformAdapter, PlatformCapabilities, PlatformRequestContext,
    PublishRequest, RemoteContent, RemoteMessage, ReplyRequest, SyncPage,
};
use crate::domain::{CapabilityState, Platform};
use crate::error::{AppError, AppResult};

use super::http::HttpTransport;

#[derive(Debug, Clone)]
pub struct XAdapter {
    transport: HttpTransport,
    base_url: String,
}

impl XAdapter {
    pub fn new(transport: HttpTransport, base_url: impl Into<String>) -> Self {
        Self {
            transport,
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl PlatformAdapter for XAdapter {
    fn platform(&self) -> Platform {
        Platform::X
    }

    fn capabilities(&self, scopes: &[String]) -> PlatformCapabilities {
        let has = |scope: &str| scopes.iter().any(|value| value == scope);
        PlatformCapabilities {
            authenticate: CapabilityState::Supported,
            publish: if has("tweet.write") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            read_own_content: if has("tweet.read") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            metrics: if has("tweet.read") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            reply: if has("tweet.write") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            direct_messages: if has("dm.read") || has("dm.write") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            detail: Some(
                "Replies are limited to interactions eligible under the X API.".to_owned(),
            ),
        }
    }

    async fn publish(
        &self,
        context: &PlatformRequestContext,
        request: PublishRequest,
    ) -> AppResult<RemoteContent> {
        let mut body = serde_json::json!({ "text": request.body });
        if let Some(parent) = request.reply_to_id {
            body["reply"] = serde_json::json!({ "in_reply_to_tweet_id": parent });
        }
        let response = self
            .transport
            .json(
                Method::POST,
                &format!("{}/2/tweets", self.base_url),
                &context.access_token,
                &[],
                Some(&body),
            )
            .await?;
        let data = response.body.get("data").unwrap_or(&response.body);
        let id = data
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::Platform("X response did not include a post id".to_owned()))?;
        Ok(RemoteContent {
            platform: Platform::X,
            remote_id: id.to_owned(),
            body: data
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            remote_url: Some(format!(
                "https://x.com/{}/status/{id}",
                context.display_name.trim_start_matches('@')
            )),
        })
    }

    async fn sync(
        &self,
        context: &PlatformRequestContext,
        cursor: Option<&str>,
    ) -> AppResult<SyncPage> {
        let mut url = url::Url::parse(&format!(
            "{}/2/users/{}/tweets",
            self.base_url, context.account_id
        ))
        .map_err(|error| AppError::Platform(error.to_string()))?;
        url.query_pairs_mut()
            .append_pair("tweet.fields", "created_at,public_metrics");
        if let Some(cursor) = cursor {
            url.query_pairs_mut()
                .append_pair("pagination_token", cursor);
        }
        let response = self
            .transport
            .json(Method::GET, url.as_str(), &context.access_token, &[], None)
            .await?;
        let items = response
            .body
            .get("data")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| {
                let id = item.get("id")?.as_str()?.to_owned();
                Some(RemoteContent {
                    platform: Platform::X,
                    remote_url: Some(format!(
                        "https://x.com/{}/status/{id}",
                        context.display_name.trim_start_matches('@')
                    )),
                    remote_id: id,
                    body: item
                        .get("text")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                })
            })
            .collect();
        let next_cursor = response
            .body
            .pointer("/meta/next_token")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        Ok(SyncPage { items, next_cursor })
    }

    async fn reply(
        &self,
        context: &PlatformRequestContext,
        request: ReplyRequest,
    ) -> AppResult<RemoteMessage> {
        let content = self
            .publish(
                context,
                PublishRequest {
                    body: request.body.clone(),
                    title: None,
                    destination: None,
                    reply_to_id: Some(request.remote_parent_id),
                    idempotency_key: request.idempotency_key,
                },
            )
            .await?;
        Ok(RemoteMessage {
            platform: Platform::X,
            remote_id: content.remote_id,
            conversation_id: None,
            body: request.body,
        })
    }

    async fn send_direct_message(
        &self,
        context: &PlatformRequestContext,
        request: DirectMessageRequest,
    ) -> AppResult<RemoteMessage> {
        let response = self
            .transport
            .json(
                Method::POST,
                &format!(
                    "{}/2/dm_conversations/with/{}/messages",
                    self.base_url, request.recipient_id
                ),
                &context.access_token,
                &[],
                Some(&serde_json::json!({ "text": request.body })),
            )
            .await?;
        let data = response.body.get("data").unwrap_or(&response.body);
        let id = data
            .get("dm_event_id")
            .or_else(|| data.get("id"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                AppError::Platform("X response did not include a DM event id".to_owned())
            })?;
        Ok(RemoteMessage {
            platform: Platform::X,
            remote_id: id.to_owned(),
            conversation_id: data
                .get("dm_conversation_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            body: request.body,
        })
    }
}
