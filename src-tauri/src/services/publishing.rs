use chrono::Utc;
use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::Row as _;
use uuid::Uuid;

use crate::adapters::platform::{
    PlatformRegistry, PlatformRequestContext, PublishRequest, RemoteContent,
};
use crate::db::repositories::audit::AuditRepository;
use crate::db::repositories::platform::PlatformRepository;
use crate::domain::Platform;
use crate::error::{AppError, AppResult};
use crate::secrets::SecretStore;
use crate::validation::payload_hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenEnvelope {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PublishingService {
    pool: sqlx::SqlitePool,
    platforms: PlatformRegistry,
}

impl PublishingService {
    pub fn new(pool: sqlx::SqlitePool, platforms: PlatformRegistry) -> Self {
        Self { pool, platforms }
    }

    pub async fn publish(
        &self,
        secrets: &dyn SecretStore,
        account_id: Uuid,
        approval_id: Uuid,
        variant_id: Uuid,
        mut request: PublishRequest,
    ) -> AppResult<RemoteContent> {
        let account = PlatformRepository::new(self.pool.clone())
            .get(account_id)
            .await?;
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query("SELECT a.payload_hash, a.idempotency_key, a.consumed_at, a.invalidated_at, v.platform, v.body FROM approvals a JOIN content_variants v ON v.id = a.subject_id WHERE a.id = ? AND v.id = ?")
            .bind(approval_id.to_string())
            .bind(variant_id.to_string())
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or_else(|| AppError::NotFound("approval or variant".to_owned()))?;
        let platform = Platform::parse(row.try_get("platform")?)?;
        let stored_body: String = row.try_get("body")?;
        let stored_hash: String = row.try_get("payload_hash")?;
        if platform != account.platform
            || stored_hash != payload_hash(&request.body)
            || stored_body != request.body
        {
            return Err(AppError::Validation(
                "approval does not match this account and exact revision".to_owned(),
            ));
        }
        if row.try_get::<Option<String>, _>("consumed_at")?.is_some()
            || row
                .try_get::<Option<String>, _>("invalidated_at")?
                .is_some()
        {
            return Err(AppError::Validation(
                "approval has already been consumed or invalidated".to_owned(),
            ));
        }
        request.idempotency_key = row.try_get("idempotency_key")?;
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE approvals SET consumed_at = ? WHERE id = ? AND consumed_at IS NULL")
            .bind(&now)
            .bind(approval_id.to_string())
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE content_variants SET status = 'publishing' WHERE id = ?")
            .bind(variant_id.to_string())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;

        let secret = secrets.load(&account.secret_ref)?.ok_or_else(|| {
            AppError::Authentication("platform token is missing from the OS keyring".to_owned())
        })?;
        let token: TokenEnvelope = serde_json::from_str(secret.expose_secret())?;
        let context = PlatformRequestContext {
            access_token: token.access_token,
            account_id: account.remote_account_id.clone(),
            display_name: account.display_name.clone(),
            scopes: account.scopes.clone(),
        };
        let result = self
            .platforms
            .get(platform)
            .publish(&context, request.clone())
            .await;
        match result {
            Ok(remote) => {
                let mut transaction = self.pool.begin().await?;
                sqlx::query("UPDATE content_variants SET status = 'published' WHERE id = ?")
                    .bind(variant_id.to_string())
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("INSERT INTO remote_content (id, account_id, platform, remote_id, content_type, body, remote_url, author_remote_id, published_at, collected_at) VALUES (?, ?, ?, ?, 'post', ?, ?, ?, ?, ?) ON CONFLICT(platform, account_id, remote_id) DO NOTHING")
                    .bind(Uuid::new_v4().to_string())
                    .bind(account.id.to_string())
                    .bind(platform.as_str())
                    .bind(&remote.remote_id)
                    .bind(&remote.body)
                    .bind(&remote.remote_url)
                    .bind(&account.remote_account_id)
                    .bind(&now)
                    .bind(&now)
                    .execute(&mut *transaction)
                    .await?;
                transaction.commit().await?;
                AuditRepository::new(self.pool.clone())
                    .record(
                        "external_write_succeeded",
                        Some("content_variant"),
                        Some(&variant_id.to_string()),
                        &serde_json::json!({"platform": platform, "remoteId": remote.remote_id}),
                    )
                    .await?;
                Ok(remote)
            }
            Err(error) => {
                sqlx::query("UPDATE content_variants SET status = 'failed' WHERE id = ?")
                    .bind(variant_id.to_string())
                    .execute(&self.pool)
                    .await?;
                AuditRepository::new(self.pool.clone())
                    .record(
                        "external_write_failed",
                        Some("content_variant"),
                        Some(&variant_id.to_string()),
                        &serde_json::json!({"platform": platform, "errorCode": error_code(&error)}),
                    )
                    .await?;
                Err(error)
            }
        }
    }

    pub fn token_secret(
        access_token: String,
        refresh_token: Option<String>,
    ) -> AppResult<SecretString> {
        Ok(SecretString::from(serde_json::to_string(&TokenEnvelope {
            access_token,
            refresh_token,
        })?))
    }

    pub fn access_token(secret: &SecretString) -> AppResult<String> {
        Ok(serde_json::from_str::<TokenEnvelope>(secret.expose_secret())?.access_token)
    }
}

fn error_code(error: &AppError) -> &'static str {
    match error {
        AppError::Authentication(_) => "authentication",
        AppError::Permission(_) => "permission",
        AppError::Timeout(_) => "timeout",
        AppError::Unsupported(_) => "unsupported",
        _ => "platform",
    }
}
