use chrono::{DateTime, Utc};
use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::domain::icp::{IcpEvidence, IcpHypothesisDraft, IcpStatus, StoredIcpHypothesis};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct IcpRepository {
    pool: SqlitePool,
}

impl IcpRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save_hypothesis(
        &self,
        founder_id: Uuid,
        draft: IcpHypothesisDraft,
    ) -> AppResult<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool.begin().await?;
        let version: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM icp_hypotheses WHERE founder_id = ?",
        )
        .bind(founder_id.to_string())
        .fetch_one(&mut *transaction)
        .await?;
        let parent_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM icp_hypotheses WHERE founder_id = ? AND status = 'active' ORDER BY version DESC LIMIT 1",
        )
        .bind(founder_id.to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        sqlx::query("INSERT INTO icp_hypotheses (id, founder_id, version, parent_id, role, situation, urgent_problem, current_workaround, desired_outcome, objections_json, language_json, confidence, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'proposed', ?, ?)")
            .bind(id.to_string())
            .bind(founder_id.to_string())
            .bind(version)
            .bind(parent_id)
            .bind(draft.role)
            .bind(draft.situation)
            .bind(draft.urgent_problem)
            .bind(draft.current_workaround)
            .bind(draft.desired_outcome)
            .bind(serde_json::to_string(&draft.objections)?)
            .bind(serde_json::to_string(&draft.language)?)
            .bind(draft.confidence.clamp(0.0, 1.0))
            .bind(&now)
            .bind(&now)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(id)
    }

    pub async fn add_evidence(&self, evidence: IcpEvidence) -> AppResult<()> {
        sqlx::query("INSERT INTO icp_evidence (id, hypothesis_id, source_type, source_id, summary, direction, weight, accepted, created_at) VALUES (?, ?, 'manual', NULL, ?, ?, ?, ?, ?)")
            .bind(evidence.id.to_string())
            .bind(evidence.hypothesis_id.to_string())
            .bind(evidence.summary)
            .bind(match evidence.direction {
                crate::domain::icp::EvidenceDirection::Supports => "supports",
                crate::domain::icp::EvidenceDirection::Contradicts => "contradicts",
                crate::domain::icp::EvidenceDirection::Neutral => "neutral",
            })
            .bind(evidence.weight.clamp(0.0, 1.0))
            .bind(i64::from(evidence.accepted))
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_for_founder(&self, founder_id: Uuid) -> AppResult<Vec<StoredIcpHypothesis>> {
        let rows = sqlx::query("SELECT id, founder_id, version, parent_id, role, situation, urgent_problem, current_workaround, desired_outcome, objections_json, language_json, confidence, status, created_at, updated_at FROM icp_hypotheses WHERE founder_id = ? AND status != 'archived' ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END, version DESC")
            .bind(founder_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_hypothesis).collect()
    }

    pub async fn accept(&self, founder_id: Uuid, hypothesis_id: Uuid) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool.begin().await?;
        sqlx::query("UPDATE icp_hypotheses SET status = 'archived', updated_at = ? WHERE founder_id = ? AND status = 'active' AND id != ?")
            .bind(&now)
            .bind(founder_id.to_string())
            .bind(hypothesis_id.to_string())
            .execute(&mut *transaction)
            .await?;
        let result = sqlx::query(
            "UPDATE icp_hypotheses SET status = 'active', updated_at = ? WHERE id = ? AND founder_id = ? AND status IN ('proposed', 'active')",
        )
        .bind(&now)
        .bind(hypothesis_id.to_string())
        .bind(founder_id.to_string())
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(AppError::NotFound(format!(
                "ICP hypothesis {hypothesis_id}"
            )));
        }
        transaction.commit().await?;
        Ok(())
    }
}

fn row_to_hypothesis(row: &sqlx::sqlite::SqliteRow) -> AppResult<StoredIcpHypothesis> {
    Ok(StoredIcpHypothesis {
        id: parse_uuid(row.try_get("id")?)?,
        founder_id: parse_uuid(row.try_get("founder_id")?)?,
        version: u32::try_from(row.try_get::<i64, _>("version")?)
            .map_err(|_| AppError::Internal("ICP version is outside u32".to_owned()))?,
        parent_id: row
            .try_get::<Option<&str>, _>("parent_id")?
            .map(parse_uuid)
            .transpose()?,
        draft: IcpHypothesisDraft {
            role: row.try_get("role")?,
            situation: row.try_get("situation")?,
            urgent_problem: row.try_get("urgent_problem")?,
            current_workaround: row.try_get("current_workaround")?,
            desired_outcome: row.try_get("desired_outcome")?,
            objections: serde_json::from_str(row.try_get("objections_json")?)?,
            language: serde_json::from_str(row.try_get("language_json")?)?,
            confidence: row.try_get("confidence")?,
        },
        status: IcpStatus::parse(row.try_get("status")?)?,
        created_at: parse_time(row.try_get("created_at")?)?,
        updated_at: parse_time(row.try_get("updated_at")?)?,
    })
}

fn parse_uuid(value: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value).map_err(|error| AppError::Internal(error.to_string()))
}

fn parse_time(value: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::db::repositories::founder::FounderRepository;
    use crate::domain::founder::FounderProfileInput;
    use crate::domain::icp::{IcpHypothesisDraft, IcpStatus};

    use super::IcpRepository;

    #[tokio::test]
    async fn founder_explicitly_activates_a_proposed_hypothesis() {
        let database = Database::in_memory().await.expect("database");
        let founder = FounderRepository::new(database.pool().clone())
            .save(FounderProfileInput {
                name: "Duc".to_owned(),
                product_name: "Lab".to_owned(),
                offer: "Sustainable growth".to_owned(),
                expertise: "Local-first products".to_owned(),
                goals: vec!["Qualified conversations".to_owned()],
                boundaries: vec!["No spam".to_owned()],
            })
            .await
            .expect("founder");
        let repository = IcpRepository::new(database.pool().clone());
        let id = repository
            .save_hypothesis(
                founder.id,
                IcpHypothesisDraft {
                    role: "Solo SaaS founder".to_owned(),
                    situation: "Early traction".to_owned(),
                    urgent_problem: "Inconsistent learning".to_owned(),
                    current_workaround: "Ad hoc posting".to_owned(),
                    desired_outcome: "Repeatable conversations".to_owned(),
                    objections: vec!["More busywork".to_owned()],
                    language: vec!["learning loop".to_owned()],
                    confidence: 0.45,
                },
            )
            .await
            .expect("hypothesis");
        assert_eq!(
            repository.list_for_founder(founder.id).await.expect("list")[0].status,
            IcpStatus::Proposed
        );
        repository.accept(founder.id, id).await.expect("accept");
        let first = repository.list_for_founder(founder.id).await.expect("list");
        assert_eq!(first[0].status, IcpStatus::Active);
        assert_eq!(first[0].version, 1);

        let second_id = repository
            .save_hypothesis(
                founder.id,
                IcpHypothesisDraft {
                    role: "Technical solo founder".to_owned(),
                    situation: "Building in public".to_owned(),
                    urgent_problem: "Weak audience feedback".to_owned(),
                    current_workaround: "Broad content".to_owned(),
                    desired_outcome: "Qualified conversations".to_owned(),
                    objections: vec![],
                    language: vec!["operator signal".to_owned()],
                    confidence: 0.6,
                },
            )
            .await
            .expect("second hypothesis");
        repository
            .accept(founder.id, second_id)
            .await
            .expect("accept second");
        let current = repository.list_for_founder(founder.id).await.expect("list");
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].id, second_id);
        assert_eq!(current[0].version, 2);
        assert_eq!(current[0].parent_id, Some(id));
    }
}
