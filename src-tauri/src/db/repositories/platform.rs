use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::adapters::platform::PlatformCapabilities;
use crate::domain::Platform;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedAccount {
    pub id: Uuid,
    pub platform: Platform,
    pub client_id: String,
    pub remote_account_id: String,
    pub display_name: String,
    pub secret_ref: String,
    pub scopes: Vec<String>,
    pub capabilities: PlatformCapabilities,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct PlatformRepository {
    pool: SqlitePool,
}

impl PlatformRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, account: &ConnectedAccount) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO connected_accounts (id, platform, client_id, remote_account_id, display_name, secret_ref, scopes_json, capabilities_json, token_expires_at, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(platform, remote_account_id) DO UPDATE SET client_id = excluded.client_id, display_name = excluded.display_name, secret_ref = excluded.secret_ref, scopes_json = excluded.scopes_json, capabilities_json = excluded.capabilities_json, token_expires_at = excluded.token_expires_at, status = excluded.status, updated_at = excluded.updated_at")
            .bind(account.id.to_string())
            .bind(account.platform.as_str())
            .bind(&account.client_id)
            .bind(&account.remote_account_id)
            .bind(&account.display_name)
            .bind(&account.secret_ref)
            .bind(serde_json::to_string(&account.scopes)?)
            .bind(serde_json::to_string(&account.capabilities)?)
            .bind(account.token_expires_at.map(|value| value.to_rfc3339()))
            .bind(&account.status)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list(&self) -> AppResult<Vec<ConnectedAccount>> {
        let rows = sqlx::query("SELECT id, platform, client_id, remote_account_id, display_name, secret_ref, scopes_json, capabilities_json, token_expires_at, status FROM connected_accounts WHERE status != 'revoked' AND client_id NOT IN ('goalbar-email-notifications', 'goalbar-browser-inbox') ORDER BY platform")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_account).collect()
    }

    pub async fn get(&self, id: Uuid) -> AppResult<ConnectedAccount> {
        let row = sqlx::query("SELECT id, platform, client_id, remote_account_id, display_name, secret_ref, scopes_json, capabilities_json, token_expires_at, status FROM connected_accounts WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("account {id}")))?;
        row_to_account(&row)
    }

    pub async fn mark_revoked(&self, id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE connected_accounts SET status = 'revoked', updated_at = ? WHERE id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn row_to_account(row: &sqlx::sqlite::SqliteRow) -> AppResult<ConnectedAccount> {
    let expires: Option<String> = row.try_get("token_expires_at")?;
    Ok(ConnectedAccount {
        id: Uuid::parse_str(row.try_get("id")?)
            .map_err(|error| AppError::Internal(error.to_string()))?,
        platform: Platform::parse(row.try_get("platform")?)?,
        client_id: row.try_get("client_id")?,
        remote_account_id: row.try_get("remote_account_id")?,
        display_name: row.try_get("display_name")?,
        secret_ref: row.try_get("secret_ref")?,
        scopes: serde_json::from_str(row.try_get("scopes_json")?)?,
        capabilities: serde_json::from_str(row.try_get("capabilities_json")?)?,
        token_expires_at: expires
            .map(|value| {
                DateTime::parse_from_rfc3339(&value)
                    .map(|value| value.with_timezone(&Utc))
                    .map_err(|error| AppError::Internal(error.to_string()))
            })
            .transpose()?,
        status: row.try_get("status")?,
    })
}
