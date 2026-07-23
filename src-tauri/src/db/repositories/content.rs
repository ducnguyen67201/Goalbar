use chrono::Utc;
use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::domain::Platform;
use crate::domain::approval::Approval;
use crate::domain::content::{ContentIdeaInput, GeneratedContentSet, StoredContentVariant};
use crate::error::{AppError, AppResult};
use crate::validation::require_non_empty;

#[derive(Debug, Clone)]
pub struct ContentRepository {
    pool: SqlitePool,
}

impl ContentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save_generated_set(
        &self,
        founder_id: Uuid,
        input: ContentIdeaInput,
        set: GeneratedContentSet,
        provider: &str,
        provider_version: &str,
    ) -> AppResult<Uuid> {
        if !set.has_all_platforms() {
            return Err(AppError::Validation(
                "agent response must contain all three platforms".to_owned(),
            ));
        }
        let title = require_non_empty(&input.title, "title", 200)?;
        let insight = require_non_empty(&input.insight, "insight", 10_000)?;
        let idea_id = Uuid::new_v4();
        let experiment_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT INTO content_ideas (id, founder_id, title, insight, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(idea_id.to_string())
            .bind(founder_id.to_string())
            .bind(title)
            .bind(insight)
            .bind(&now)
            .bind(&now)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("INSERT INTO experiments (id, idea_id, hypothesis, success_metric, window_days, status, created_at, updated_at) VALUES (?, ?, ?, ?, 7, 'draft', ?, ?)")
            .bind(experiment_id.to_string())
            .bind(idea_id.to_string())
            .bind(require_non_empty(&input.hypothesis, "hypothesis", 2_000)?)
            .bind(require_non_empty(&input.success_metric, "success metric", 1_000)?)
            .bind(&now)
            .bind(&now)
            .execute(&mut *transaction)
            .await?;
        for variant in set.variants {
            sqlx::query("INSERT INTO content_variants (id, idea_id, experiment_id, platform, revision, body, metadata_json, provider, provider_version, status, created_at) VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?, 'draft', ?)")
                .bind(Uuid::new_v4().to_string())
                .bind(idea_id.to_string())
                .bind(experiment_id.to_string())
                .bind(variant.platform.as_str())
                .bind(require_non_empty(&variant.body, "variant body", 40_000)?)
                .bind(serde_json::json!({"rationale": variant.rationale, "callToAction": variant.call_to_action}).to_string())
                .bind(provider)
                .bind(provider_version)
                .bind(&now)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(idea_id)
    }

    pub async fn approve_variant(&self, variant_id: Uuid, body: &str) -> AppResult<Approval> {
        let approval = Approval::new("content_variant", variant_id, body);
        let mut transaction = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE content_variants SET status = 'approved' WHERE id = ? AND body = ?",
        )
        .bind(variant_id.to_string())
        .bind(body)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        if updated != 1 {
            return Err(AppError::Validation(
                "variant changed or does not exist".to_owned(),
            ));
        }
        sqlx::query("INSERT INTO approvals (id, subject_type, subject_id, payload_hash, idempotency_key, approved_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(approval.id.to_string())
            .bind(&approval.subject_type)
            .bind(approval.subject_id.to_string())
            .bind(&approval.payload_hash)
            .bind(approval.idempotency_key.to_string())
            .bind(approval.approved_at.to_rfc3339())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(approval)
    }

    pub async fn variants_for_idea(&self, idea_id: Uuid) -> AppResult<Vec<StoredContentVariant>> {
        let rows = sqlx::query("SELECT id, platform, revision, body, status FROM content_variants WHERE idea_id = ? ORDER BY platform, revision DESC")
            .bind(idea_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|row| {
                Ok(StoredContentVariant {
                    id: Uuid::parse_str(row.try_get("id")?)
                        .map_err(|error| AppError::Internal(error.to_string()))?,
                    platform: Platform::parse(row.try_get("platform")?)?,
                    revision: row.try_get::<i64, _>("revision")? as u32,
                    body: row.try_get("body")?,
                    status: row.try_get("status")?,
                })
            })
            .collect()
    }
}
