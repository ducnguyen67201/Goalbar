use async_trait::async_trait;
use reqwest::Method;

use super::http::HttpTransport;
use super::{
    DirectMessageRequest, PlatformAdapter, PlatformCapabilities, PlatformRequestContext,
    PublishRequest, RemoteContent, RemoteMessage, ReplyRequest, SyncPage,
};
use crate::domain::{CapabilityState, Platform};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct RedditAdapter {
    transport: HttpTransport,
    base_url: String,
}

impl RedditAdapter {
    pub fn new(transport: HttpTransport, base_url: impl Into<String>) -> Self {
        Self {
            transport,
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl PlatformAdapter for RedditAdapter {
    fn platform(&self) -> Platform {
        Platform::Reddit
    }

    fn capabilities(&self, scopes: &[String]) -> PlatformCapabilities {
        let has = |scope: &str| scopes.iter().any(|value| value == scope);
        PlatformCapabilities {
            authenticate: CapabilityState::ApprovalPending,
            publish: if has("submit") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            read_own_content: if has("history") || has("read") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            metrics: if has("history") || has("read") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            reply: if has("submit") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            direct_messages: if has("privatemessages") {
                CapabilityState::Supported
            } else {
                CapabilityState::ApprovalPending
            },
            detail: Some(
                "Requires approved Data API access and current data-use terms.".to_owned(),
            ),
        }
    }

    async fn publish(
        &self,
        context: &PlatformRequestContext,
        request: PublishRequest,
    ) -> AppResult<RemoteContent> {
        let subreddit = request.destination.ok_or_else(|| {
            AppError::Validation("Reddit destination subreddit is required".to_owned())
        })?;
        let title = request
            .title
            .ok_or_else(|| AppError::Validation("Reddit title is required".to_owned()))?;
        let form = vec![
            ("api_type".to_owned(), "json".to_owned()),
            ("kind".to_owned(), "self".to_owned()),
            ("sr".to_owned(), subreddit),
            ("title".to_owned(), title),
            ("text".to_owned(), request.body.clone()),
            ("raw_json".to_owned(), "1".to_owned()),
        ];
        let response = self
            .transport
            .form(
                Method::POST,
                &format!("{}/api/submit", self.base_url),
                &context.access_token,
                &form,
            )
            .await?;
        let errors = response
            .body
            .pointer("/json/errors")
            .and_then(serde_json::Value::as_array);
        if errors.is_some_and(|errors| !errors.is_empty()) {
            return Err(AppError::Platform(format!(
                "Reddit rejected the post: {}",
                response.body
            )));
        }
        let url = response
            .body
            .pointer("/json/data/url")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let id = response
            .body
            .pointer("/json/data/name")
            .or_else(|| response.body.pointer("/json/data/id"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                AppError::Platform("Reddit response did not include a post id".to_owned())
            })?;
        Ok(RemoteContent {
            platform: Platform::Reddit,
            remote_id: id.to_owned(),
            body: request.body,
            remote_url: url,
        })
    }

    async fn sync(
        &self,
        context: &PlatformRequestContext,
        cursor: Option<&str>,
    ) -> AppResult<SyncPage> {
        let mut url = url::Url::parse(&format!(
            "{}/user/{}/submitted",
            self.base_url,
            context.display_name.trim_start_matches("u/")
        ))
        .map_err(|error| AppError::Platform(error.to_string()))?;
        url.query_pairs_mut()
            .append_pair("raw_json", "1")
            .append_pair("limit", "25");
        if let Some(cursor) = cursor {
            url.query_pairs_mut().append_pair("after", cursor);
        }
        let response = self
            .transport
            .json(Method::GET, url.as_str(), &context.access_token, &[], None)
            .await?;
        let items = response
            .body
            .pointer("/data/children")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|child| child.get("data"))
            .filter_map(|item| {
                Some(RemoteContent {
                    platform: Platform::Reddit,
                    remote_id: item
                        .get("name")
                        .or_else(|| item.get("id"))?
                        .as_str()?
                        .to_owned(),
                    body: item
                        .get("selftext")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    remote_url: item
                        .get("permalink")
                        .and_then(serde_json::Value::as_str)
                        .map(|path| format!("https://www.reddit.com{path}")),
                })
            })
            .collect();
        let next_cursor = response
            .body
            .pointer("/data/after")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        Ok(SyncPage { items, next_cursor })
    }

    async fn reply(
        &self,
        context: &PlatformRequestContext,
        request: ReplyRequest,
    ) -> AppResult<RemoteMessage> {
        let form = vec![
            ("api_type".to_owned(), "json".to_owned()),
            ("thing_id".to_owned(), request.remote_parent_id.clone()),
            ("text".to_owned(), request.body.clone()),
            ("raw_json".to_owned(), "1".to_owned()),
        ];
        let response = self
            .transport
            .form(
                Method::POST,
                &format!("{}/api/comment", self.base_url),
                &context.access_token,
                &form,
            )
            .await?;
        let id = response
            .body
            .pointer("/json/data/things/0/data/name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("pending");
        Ok(RemoteMessage {
            platform: Platform::Reddit,
            remote_id: id.to_owned(),
            conversation_id: Some(request.remote_parent_id),
            body: request.body,
        })
    }

    async fn send_direct_message(
        &self,
        context: &PlatformRequestContext,
        request: DirectMessageRequest,
    ) -> AppResult<RemoteMessage> {
        let form = vec![
            ("api_type".to_owned(), "json".to_owned()),
            ("to".to_owned(), request.recipient_id.clone()),
            ("subject".to_owned(), "Goalbar conversation".to_owned()),
            ("text".to_owned(), request.body.clone()),
            ("raw_json".to_owned(), "1".to_owned()),
        ];
        let response = self
            .transport
            .form(
                Method::POST,
                &format!("{}/api/compose", self.base_url),
                &context.access_token,
                &form,
            )
            .await?;
        let errors = response
            .body
            .pointer("/json/errors")
            .and_then(serde_json::Value::as_array);
        if errors.is_some_and(|errors| !errors.is_empty()) {
            return Err(AppError::Platform(format!(
                "Reddit rejected the message: {}",
                response.body
            )));
        }
        Ok(RemoteMessage {
            platform: Platform::Reddit,
            remote_id: request.idempotency_key,
            conversation_id: None,
            body: request.body,
        })
    }
}
