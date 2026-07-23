use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use base64::Engine as _;
use chrono::{DateTime, Utc};
use rand::random;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpListener;
use url::Url;
use uuid::Uuid;

use crate::domain::Platform;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BeginOAuthRequest {
    pub platform: Platform,
    pub client_id: String,
    pub remote_account_id: String,
    pub display_name: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginOAuthResponse {
    pub session_id: Uuid,
    pub authorization_url: String,
    pub redirect_uri: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthStatus {
    WaitingForBrowser,
    CodeReceived,
    Complete,
    Failed,
    Expired,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStatusResponse {
    pub session_id: Uuid,
    pub status: OAuthStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompletedOAuth {
    pub request: BeginOAuthRequest,
    pub token: OAuthToken,
}

#[derive(Debug, Clone)]
struct PendingOAuth {
    request: BeginOAuthRequest,
    redirect_uri: String,
    state: String,
    verifier: String,
    code: Option<String>,
    status: OAuthStatus,
    error: Option<String>,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OAuthManager {
    pending: Arc<Mutex<HashMap<Uuid, PendingOAuth>>>,
    client: Client,
}

impl Default for OAuthManager {
    fn default() -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
            client: Client::new(),
        }
    }
}

impl OAuthManager {
    pub async fn begin(
        &self,
        request: BeginOAuthRequest,
        open_browser: bool,
    ) -> AppResult<BeginOAuthResponse> {
        if request.client_id.trim().is_empty() {
            return Err(AppError::Validation(
                "platform client ID is required".to_owned(),
            ));
        }
        if request.remote_account_id.trim().is_empty() || request.display_name.trim().is_empty() {
            return Err(AppError::Validation(
                "account ID and display name are required".to_owned(),
            ));
        }
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let port = listener.local_addr()?.port();
        let redirect_uri = format!("http://127.0.0.1:{port}/oauth/callback");
        let state = random_urlsafe(32);
        let verifier = random_urlsafe(64);
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(verifier.as_bytes()));
        let authorization_url = authorization_url(&request, &redirect_uri, &state, &challenge)?;
        let session_id = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::minutes(3);
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                session_id,
                PendingOAuth {
                    request,
                    redirect_uri: redirect_uri.clone(),
                    state,
                    verifier,
                    code: None,
                    status: OAuthStatus::WaitingForBrowser,
                    error: None,
                    expires_at,
                },
            );
        let pending = self.pending.clone();
        tokio::spawn(async move {
            handle_callback(listener, pending, session_id).await;
        });
        if open_browser {
            open::that(&authorization_url)
                .map_err(|error| AppError::Io(std::io::Error::other(error.to_string())))?;
        }
        Ok(BeginOAuthResponse {
            session_id,
            authorization_url,
            redirect_uri,
            expires_at,
        })
    }

    pub fn status(&self, session_id: Uuid) -> AppResult<OAuthStatusResponse> {
        let pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        let item = pending
            .get(&session_id)
            .ok_or_else(|| AppError::NotFound(format!("OAuth session {session_id}")))?;
        Ok(OAuthStatusResponse {
            session_id,
            status: item.status,
            error: item.error.clone(),
        })
    }

    pub async fn complete(&self, session_id: Uuid) -> AppResult<CompletedOAuth> {
        let pending = {
            let values = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
            values
                .get(&session_id)
                .cloned()
                .ok_or_else(|| AppError::NotFound(format!("OAuth session {session_id}")))?
        };
        if pending.expires_at < Utc::now() {
            self.set_failure(session_id, OAuthStatus::Expired, "OAuth session expired");
            return Err(AppError::Timeout("OAuth session expired".to_owned()));
        }
        let code = pending
            .code
            .as_deref()
            .ok_or_else(|| AppError::Validation("OAuth callback has not arrived yet".to_owned()))?;
        let token = self.exchange(&pending, code).await?;
        if let Some(value) = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(&session_id)
        {
            value.status = OAuthStatus::Complete;
            value.code = None;
            value.verifier.clear();
            value.state.clear();
        }
        Ok(CompletedOAuth {
            request: pending.request,
            token,
        })
    }

    fn set_failure(&self, session_id: Uuid, status: OAuthStatus, error: &str) {
        if let Some(value) = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(&session_id)
        {
            value.status = status;
            value.error = Some(error.to_owned());
            value.code = None;
            value.verifier.clear();
        }
    }

    async fn exchange(&self, pending: &PendingOAuth, code: &str) -> AppResult<OAuthToken> {
        let (url, mut request) = match pending.request.platform {
            Platform::X => (
                "https://api.x.com/2/oauth2/token",
                self.client.post("https://api.x.com/2/oauth2/token"),
            ),
            Platform::Reddit => (
                "https://www.reddit.com/api/v1/access_token",
                self.client
                    .post("https://www.reddit.com/api/v1/access_token")
                    .basic_auth(&pending.request.client_id, Some("")),
            ),
            Platform::Linkedin => (
                "https://www.linkedin.com/oauth/v2/accessToken",
                self.client
                    .post("https://www.linkedin.com/oauth/v2/accessToken"),
            ),
        };
        let mut form = vec![
            ("grant_type", "authorization_code".to_owned()),
            ("code", code.to_owned()),
            ("redirect_uri", pending.redirect_uri.clone()),
        ];
        if pending.request.platform != Platform::Reddit {
            form.push(("client_id", pending.request.client_id.clone()));
            form.push(("code_verifier", pending.verifier.clone()));
        }
        request = request.form(&form);
        let response = request
            .send()
            .await
            .map_err(|error| AppError::Platform(format!("token exchange failed: {error}")))?;
        let status = response.status();
        let value: serde_json::Value = response.json().await.map_err(|error| {
            AppError::Platform(format!("invalid token response from {url}: {error}"))
        })?;
        if !status.is_success() {
            return Err(AppError::Authentication(format!(
                "token endpoint returned {status}: {value}"
            )));
        }
        let access_token = value
            .get("access_token")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                AppError::Authentication(
                    "token response did not contain an access token".to_owned(),
                )
            })?
            .to_owned();
        let refresh_token = value
            .get("refresh_token")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let expires_at = value
            .get("expires_in")
            .and_then(serde_json::Value::as_i64)
            .map(|seconds| Utc::now() + chrono::Duration::seconds(seconds));
        let scopes = value
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .map(|value| {
                value
                    .split([' ', ','])
                    .filter(|item| !item.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_else(|| pending.request.scopes.clone());
        Ok(OAuthToken {
            access_token,
            refresh_token,
            expires_at,
            scopes,
        })
    }
}

fn authorization_url(
    request: &BeginOAuthRequest,
    redirect_uri: &str,
    state: &str,
    challenge: &str,
) -> AppResult<String> {
    let endpoint = match request.platform {
        Platform::X => "https://x.com/i/oauth2/authorize",
        Platform::Reddit => "https://www.reddit.com/api/v1/authorize",
        Platform::Linkedin => "https://www.linkedin.com/oauth/native-pkce/authorization",
    };
    let mut url = Url::parse(endpoint).map_err(|error| AppError::Internal(error.to_string()))?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &request.client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", state)
        .append_pair("scope", &request.scopes.join(" "));
    match request.platform {
        Platform::X | Platform::Linkedin => {
            url.query_pairs_mut()
                .append_pair("code_challenge", challenge)
                .append_pair("code_challenge_method", "S256");
        }
        Platform::Reddit => {
            url.query_pairs_mut().append_pair("duration", "permanent");
        }
    }
    Ok(url.to_string())
}

async fn handle_callback(
    listener: TcpListener,
    pending: Arc<Mutex<HashMap<Uuid, PendingOAuth>>>,
    session_id: Uuid,
) {
    let result = tokio::time::timeout(Duration::from_secs(180), listener.accept()).await;
    let Ok(Ok((mut stream, address))) = result else {
        update_callback_failure(
            &pending,
            session_id,
            OAuthStatus::Expired,
            "OAuth callback timed out",
        );
        return;
    };
    if !address.ip().is_loopback() {
        update_callback_failure(
            &pending,
            session_id,
            OAuthStatus::Failed,
            "callback was not local",
        );
        return;
    }
    let mut buffer = vec![0_u8; 16 * 1024];
    let Ok(read) = stream.read(&mut buffer).await else {
        update_callback_failure(
            &pending,
            session_id,
            OAuthStatus::Failed,
            "callback could not be read",
        );
        return;
    };
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1));
    let parsed = target.and_then(|target| Url::parse(&format!("http://127.0.0.1{target}")).ok());
    let response = if let Some(url) = parsed.filter(|url| url.path() == "/oauth/callback") {
        let values: HashMap<String, String> = url.query_pairs().into_owned().collect();
        let mut guard = pending.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(item) = guard.get_mut(&session_id) {
            if values.get("state") != Some(&item.state) {
                item.status = OAuthStatus::Failed;
                item.error = Some("OAuth state mismatch".to_owned());
                html_response(
                    401,
                    "Connection rejected",
                    "The OAuth state did not match. Return to Tagline and try again.",
                )
            } else if let Some(error) = values.get("error") {
                item.status = OAuthStatus::Failed;
                item.error = Some(error.clone());
                html_response(
                    400,
                    "Connection cancelled",
                    "The platform did not grant access. You can close this window.",
                )
            } else if let Some(code) = values.get("code") {
                item.code = Some(code.clone());
                item.status = OAuthStatus::CodeReceived;
                html_response(
                    200,
                    "Account connected",
                    "Return to Tagline to finish the connection. You can close this window.",
                )
            } else {
                item.status = OAuthStatus::Failed;
                item.error = Some("callback did not include an authorization code".to_owned());
                html_response(
                    400,
                    "Connection failed",
                    "The callback did not contain an authorization code.",
                )
            }
        } else {
            html_response(
                404,
                "Connection expired",
                "Tagline no longer has this OAuth session.",
            )
        }
    } else {
        update_callback_failure(
            &pending,
            session_id,
            OAuthStatus::Failed,
            "invalid callback path",
        );
        html_response(404, "Not found", "This callback path is not valid.")
    };
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

fn update_callback_failure(
    pending: &Arc<Mutex<HashMap<Uuid, PendingOAuth>>>,
    session_id: Uuid,
    status: OAuthStatus,
    error: &str,
) {
    if let Some(item) = pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_mut(&session_id)
    {
        item.status = status;
        item.error = Some(error.to_owned());
    }
}

fn html_response(status: u16, title: &str, message: &str) -> String {
    let status_text = if status == 200 { "OK" } else { "Error" };
    let body = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>{title}</title><style>body{{font:16px system-ui;padding:3rem;max-width:42rem;background:#10130f;color:#f3f1e8}}h1{{color:#d9ff70}}</style><h1>{title}</h1><p>{message}</p>"
    );
    format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn random_urlsafe(bytes: usize) -> String {
    let mut output = Vec::with_capacity(bytes);
    while output.len() < bytes {
        output.extend_from_slice(&random::<[u8; 32]>());
    }
    output.truncate(bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(output)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::TcpStream;
    use url::Url;

    use super::{BeginOAuthRequest, OAuthManager, OAuthStatus};
    use crate::domain::Platform;

    #[tokio::test]
    async fn local_callback_accepts_one_matching_state() {
        let manager = OAuthManager::default();
        let begun = manager
            .begin(
                BeginOAuthRequest {
                    platform: Platform::X,
                    client_id: "client".to_owned(),
                    remote_account_id: "123".to_owned(),
                    display_name: "founder".to_owned(),
                    scopes: vec!["tweet.write".to_owned()],
                },
                false,
            )
            .await
            .expect("begin");
        let url = Url::parse(&begun.authorization_url).expect("authorization URL");
        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .expect("state")
            .1
            .into_owned();
        let callback = Url::parse(&begun.redirect_uri).expect("redirect URL");
        let mut stream = TcpStream::connect(("127.0.0.1", callback.port().expect("port")))
            .await
            .expect("connect");
        let request = format!(
            "GET /oauth/callback?code=test-code&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.expect("write");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .await
            .expect("response");
        assert!(response.contains("Account connected"));
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(
            manager.status(begun.session_id).expect("status").status,
            OAuthStatus::CodeReceived
        );
    }
}
