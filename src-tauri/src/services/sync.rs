use chrono::Utc;
use secrecy::ExposeSecret as _;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::adapters::platform::{PlatformRegistry, PlatformRequestContext, SyncPage};
use crate::db::repositories::platform::PlatformRepository;
use crate::error::{AppError, AppResult};
use crate::secrets::SecretStore;

#[derive(Debug, Clone)]
pub struct SyncService {
    pool: SqlitePool,
    platforms: PlatformRegistry,
}

impl SyncService {
    pub fn new(pool: SqlitePool, platforms: PlatformRegistry) -> Self {
        Self { pool, platforms }
    }

    pub async fn sync_account(
        &self,
        secrets: &dyn SecretStore,
        account_id: Uuid,
    ) -> AppResult<SyncPage> {
        let account = PlatformRepository::new(self.pool.clone())
            .get(account_id)
            .await?;
        let cursor: Option<String> = sqlx::query_scalar(
            "SELECT cursor FROM sync_cursors WHERE account_id = ? AND resource = 'own_content'",
        )
        .bind(account.id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .flatten();
        let secret = secrets
            .load(&account.secret_ref)?
            .ok_or_else(|| AppError::Authentication("platform token is missing".to_owned()))?;
        let token: serde_json::Value = serde_json::from_str(secret.expose_secret())?;
        let access_token = token
            .get("accessToken")
            .or_else(|| token.get("access_token"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::Authentication("stored token is invalid".to_owned()))?;
        let page = self
            .platforms
            .get(account.platform)
            .sync(
                &PlatformRequestContext {
                    access_token: access_token.to_owned(),
                    account_id: account.remote_account_id.clone(),
                    display_name: account.display_name.clone(),
                    scopes: account.scopes.clone(),
                },
                cursor.as_deref(),
            )
            .await?;
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool.begin().await?;
        for item in &page.items {
            sqlx::query("INSERT INTO remote_content (id, account_id, platform, remote_id, content_type, body, remote_url, author_remote_id, published_at, collected_at) VALUES (?, ?, ?, ?, 'post', ?, ?, ?, ?, ?) ON CONFLICT(platform, account_id, remote_id) DO UPDATE SET body = excluded.body, remote_url = excluded.remote_url, collected_at = excluded.collected_at")
                .bind(Uuid::new_v4().to_string())
                .bind(account.id.to_string())
                .bind(account.platform.as_str())
                .bind(&item.remote_id)
                .bind(&item.body)
                .bind(&item.remote_url)
                .bind(&account.remote_account_id)
                .bind(&now)
                .bind(&now)
                .execute(&mut *transaction)
                .await?;
        }
        sqlx::query("INSERT INTO sync_cursors (account_id, resource, cursor, last_success_at) VALUES (?, 'own_content', ?, ?) ON CONFLICT(account_id, resource) DO UPDATE SET cursor = excluded.cursor, last_success_at = excluded.last_success_at, next_attempt_at = NULL")
            .bind(account.id.to_string())
            .bind(&page.next_cursor)
            .bind(&now)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(page)
    }
}
