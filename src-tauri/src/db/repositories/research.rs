use chrono::Utc;
use sha2::{Digest as _, Sha256};
use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::domain::Platform;
use crate::domain::browser::{
    BrowserResearchTrace, ResearchFindingDraft, ResearchFindingStatus, StoredResearchFinding,
};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct ResearchRepository {
    pool: SqlitePool,
}

impl ResearchRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn append_trace(
        &self,
        run_id: Uuid,
        step: u32,
        action: &str,
        message: &str,
        url: &str,
    ) -> AppResult<BrowserResearchTrace> {
        if !matches!(
            action,
            "observe" | "scroll" | "open_link" | "go_back" | "finish" | "pause" | "error"
        ) {
            return Err(AppError::Validation(
                "unsupported research trace action".to_owned(),
            ));
        }
        let trace = BrowserResearchTrace {
            id: Uuid::new_v4(),
            run_id,
            step,
            action: action.to_owned(),
            message: message.chars().take(1_000).collect(),
            url: url.to_owned(),
            created_at: Utc::now().to_rfc3339(),
        };
        sqlx::query("INSERT INTO browser_research_trace (id, run_id, step, action, message, url, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(trace.id.to_string())
            .bind(trace.run_id.to_string())
            .bind(i64::from(trace.step))
            .bind(&trace.action)
            .bind(&trace.message)
            .bind(&trace.url)
            .bind(&trace.created_at)
            .execute(&self.pool)
            .await?;
        Ok(trace)
    }

    pub async fn stage_findings(
        &self,
        run_id: Uuid,
        platform: Platform,
        source_url: &str,
        findings: Vec<ResearchFindingDraft>,
    ) -> AppResult<Vec<StoredResearchFinding>> {
        let mut stored = Vec::new();
        let mut transaction = self.pool.begin().await?;
        for finding in findings.into_iter().take(8) {
            let summary = bounded_required(&finding.summary, "finding summary", 500)?;
            let evidence_excerpt =
                bounded_required(&finding.evidence_excerpt, "evidence excerpt", 1_200)?;
            if !finding.confidence.is_finite() {
                return Err(AppError::Validation(
                    "finding confidence must be finite".to_owned(),
                ));
            }
            let now = Utc::now().to_rfc3339();
            let dedupe_key = finding_key(
                finding.category.as_str(),
                &summary,
                &evidence_excerpt,
                source_url,
            );
            let id = Uuid::new_v4();
            let affected = sqlx::query("INSERT INTO browser_research_findings (id, run_id, platform, category, summary, evidence_excerpt, source_url, confidence, status, dedupe_key, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'proposed', ?, ?, ?) ON CONFLICT(run_id, dedupe_key) DO NOTHING")
                .bind(id.to_string())
                .bind(run_id.to_string())
                .bind(platform.as_str())
                .bind(finding.category.as_str())
                .bind(&summary)
                .bind(&evidence_excerpt)
                .bind(source_url)
                .bind(finding.confidence.clamp(0.0, 1.0))
                .bind(dedupe_key)
                .bind(&now)
                .bind(&now)
                .execute(&mut *transaction)
                .await?
                .rows_affected();
            if affected == 1 {
                stored.push(StoredResearchFinding {
                    id,
                    run_id,
                    platform,
                    category: finding.category,
                    summary,
                    evidence_excerpt,
                    source_url: source_url.to_owned(),
                    confidence: finding.confidence.clamp(0.0, 1.0),
                    status: ResearchFindingStatus::Proposed,
                    created_at: now.clone(),
                    updated_at: now,
                });
            }
        }
        transaction.commit().await?;
        Ok(stored)
    }

    pub async fn list_findings(&self, run_id: Uuid) -> AppResult<Vec<StoredResearchFinding>> {
        let rows = sqlx::query("SELECT id, run_id, platform, category, summary, evidence_excerpt, source_url, confidence, status, created_at, updated_at FROM browser_research_findings WHERE run_id = ? ORDER BY created_at DESC")
            .bind(run_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_finding).collect()
    }

    pub async fn list_trace(&self, run_id: Uuid) -> AppResult<Vec<BrowserResearchTrace>> {
        let rows = sqlx::query("SELECT id, run_id, step, action, message, url, created_at FROM browser_research_trace WHERE run_id = ? ORDER BY step, created_at")
            .bind(run_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|row| {
                Ok(BrowserResearchTrace {
                    id: parse_uuid(row.try_get("id")?)?,
                    run_id: parse_uuid(row.try_get("run_id")?)?,
                    step: row.try_get::<i64, _>("step")? as u32,
                    action: row.try_get("action")?,
                    message: row.try_get("message")?,
                    url: row.try_get("url")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }

    pub async fn review(
        &self,
        finding_id: Uuid,
        status: ResearchFindingStatus,
    ) -> AppResult<StoredResearchFinding> {
        if status == ResearchFindingStatus::Proposed {
            return Err(AppError::Validation(
                "review must accept or reject a finding".to_owned(),
            ));
        }
        let result = sqlx::query("UPDATE browser_research_findings SET status = ?, updated_at = ? WHERE id = ? AND status = 'proposed'")
            .bind(status.as_str())
            .bind(Utc::now().to_rfc3339())
            .bind(finding_id.to_string())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() != 1 {
            return Err(AppError::NotFound(format!(
                "proposed research finding {finding_id}"
            )));
        }
        let row = sqlx::query("SELECT id, run_id, platform, category, summary, evidence_excerpt, source_url, confidence, status, created_at, updated_at FROM browser_research_findings WHERE id = ?")
            .bind(finding_id.to_string())
            .fetch_one(&self.pool)
            .await?;
        row_to_finding(&row)
    }
}

fn row_to_finding(row: &sqlx::sqlite::SqliteRow) -> AppResult<StoredResearchFinding> {
    Ok(StoredResearchFinding {
        id: parse_uuid(row.try_get("id")?)?,
        run_id: parse_uuid(row.try_get("run_id")?)?,
        platform: Platform::parse(row.try_get("platform")?)?,
        category: crate::domain::browser::ResearchFindingCategory::parse(row.try_get("category")?)?,
        summary: row.try_get("summary")?,
        evidence_excerpt: row.try_get("evidence_excerpt")?,
        source_url: row.try_get("source_url")?,
        confidence: row.try_get("confidence")?,
        status: ResearchFindingStatus::parse(row.try_get("status")?)?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn bounded_required(value: &str, label: &str, max_chars: usize) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars {
        return Err(AppError::Validation(format!(
            "{label} must contain between 1 and {max_chars} characters"
        )));
    }
    Ok(value.to_owned())
}

fn finding_key(category: &str, summary: &str, evidence: &str, source_url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(category.as_bytes());
    hasher.update(summary.as_bytes());
    hasher.update(evidence.as_bytes());
    hasher.update(source_url.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn parse_uuid(value: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value).map_err(|error| AppError::Internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::db::repositories::history::{HistoryContextPurpose, HistoryRepository};
    use crate::domain::Platform;
    use crate::domain::browser::{
        BrowserRunLimits, ResearchFindingCategory, ResearchFindingDraft, ResearchFindingStatus,
    };
    use crate::domain::history::ActivityOwnership;

    use super::ResearchRepository;

    #[tokio::test]
    async fn findings_require_review_before_they_enter_icp_context() {
        let database = Database::in_memory().await.expect("database");
        let history = HistoryRepository::new(database.pool().clone());
        let run = history
            .create_browser_run(
                Platform::Linkedin,
                ActivityOwnership::Reference,
                "Discover ICP language",
                &BrowserRunLimits {
                    maximum_items: 10,
                    maximum_steps: 3,
                    earliest_date: None,
                },
                Some("codex"),
            )
            .await
            .expect("run");
        let repository = ResearchRepository::new(database.pool().clone());
        let staged = repository
            .stage_findings(
                run.run_id,
                Platform::Linkedin,
                "https://www.linkedin.com/feed/",
                vec![ResearchFindingDraft {
                    category: ResearchFindingCategory::Pain,
                    summary: "Founders struggle to keep a consistent learning loop".to_owned(),
                    evidence_excerpt: "I never know what to post next".to_owned(),
                    confidence: 0.8,
                }],
            )
            .await
            .expect("stage");
        let before = history
            .bounded_context(HistoryContextPurpose::Icp, 10, 10_000)
            .await
            .expect("context");
        assert!(!before.to_string().contains("consistent learning loop"));

        repository
            .review(staged[0].id, ResearchFindingStatus::Accepted)
            .await
            .expect("accept");
        let after = history
            .bounded_context(HistoryContextPurpose::Icp, 10, 10_000)
            .await
            .expect("context");
        assert!(after.to_string().contains("consistent learning loop"));
        assert_eq!(after["privateMessagesIncluded"], false);
    }
}
