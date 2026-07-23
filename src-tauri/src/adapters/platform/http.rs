use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde_json::Value;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct HttpTransport {
    client: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}

impl HttpTransport {
    pub fn production() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("goalbar/0.1.0")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }

    pub async fn json(
        &self,
        method: Method,
        url: &str,
        token: &str,
        headers: &[(&str, &str)],
        body: Option<&Value>,
    ) -> AppResult<HttpResponse> {
        let mut request = self.client.request(method, url).bearer_auth(token);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        self.send(request).await
    }

    pub async fn form(
        &self,
        method: Method,
        url: &str,
        token: &str,
        form: &[(String, String)],
    ) -> AppResult<HttpResponse> {
        self.send(
            self.client
                .request(method, url)
                .bearer_auth(token)
                .form(form),
        )
        .await
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> AppResult<HttpResponse> {
        let response = request.send().await.map_err(|error| {
            if error.is_timeout() {
                AppError::Timeout("platform request timed out".to_owned())
            } else {
                AppError::Platform(error.to_string())
            }
        })?;
        let status = response.status();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.to_string(), value.to_owned()))
            })
            .collect();
        let text = response
            .text()
            .await
            .map_err(|error| AppError::Platform(error.to_string()))?;
        let body = if text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({ "message": text }))
        };
        if !status.is_success() {
            return Err(map_status(status, &body));
        }
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

fn map_status(status: StatusCode, body: &Value) -> AppError {
    let detail = body
        .get("detail")
        .or_else(|| body.get("message"))
        .or_else(|| body.get("error"))
        .map(Value::to_string)
        .unwrap_or_else(|| "platform rejected the request".to_owned());
    match status.as_u16() {
        401 => AppError::Authentication(detail),
        403 => AppError::Permission(detail),
        404 => AppError::NotFound(detail),
        408 | 504 => AppError::Timeout(detail),
        409 | 429 => AppError::Platform(format!("retryable HTTP {status}: {detail}")),
        _ => AppError::Platform(format!("HTTP {status}: {detail}")),
    }
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;

    use super::map_status;

    #[test]
    fn maps_auth_and_permission_separately() {
        assert!(matches!(
            map_status(StatusCode::UNAUTHORIZED, &serde_json::json!({})),
            crate::error::AppError::Authentication(_)
        ));
        assert!(matches!(
            map_status(StatusCode::FORBIDDEN, &serde_json::json!({})),
            crate::error::AppError::Permission(_)
        ));
    }
}
