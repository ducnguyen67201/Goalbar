use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct AuditRepository {
    pool: SqlitePool,
}

impl AuditRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn record(
        &self,
        kind: &str,
        subject_type: Option<&str>,
        subject_id: Option<&str>,
        detail: &Value,
    ) -> AppResult<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO audit_events (id, kind, subject_type, subject_id, detail_json, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(id.to_string())
            .bind(kind)
            .bind(subject_type)
            .bind(subject_id)
            .bind(detail.to_string())
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(id)
    }
}
