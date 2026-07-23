use chrono::Utc;
use serde::Serialize;
use sqlx::{Row as _, SqlitePool};

use crate::app_state::AppState;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataArtifact {
    pub path: String,
    pub kind: String,
    pub created_at: String,
    pub includes_secrets: bool,
}

pub async fn export_json(state: &AppState) -> AppResult<DataArtifact> {
    let root = state
        .database
        .path()?
        .parent()
        .ok_or_else(|| AppError::Internal("database has no parent directory".to_owned()))?;
    let directory = root.join("exports");
    tokio::fs::create_dir_all(&directory).await?;
    let created_at = Utc::now();
    let path = directory.join(format!(
        "tagline-{}.json",
        created_at.format("%Y%m%d-%H%M%S")
    ));
    let founder = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'name', name, 'productName', product_name, 'offer', offer, 'expertise', expertise, 'goals', json(goals_json), 'boundaries', json(boundaries_json), 'createdAt', created_at, 'updatedAt', updated_at)) FROM founder_profiles",
    )
    .await?;
    let icp = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'founderId', founder_id, 'role', role, 'situation', situation, 'urgentProblem', urgent_problem, 'desiredOutcome', desired_outcome, 'confidence', confidence, 'status', status, 'createdAt', created_at)) FROM icp_hypotheses",
    )
    .await?;
    let ideas = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'founderId', founder_id, 'title', title, 'insight', insight, 'createdAt', created_at)) FROM content_ideas",
    )
    .await?;
    let experiments = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'ideaId', idea_id, 'hypothesis', hypothesis, 'successMetric', success_metric, 'windowDays', window_days, 'status', status, 'createdAt', created_at)) FROM experiments",
    )
    .await?;
    let learnings = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'founderId', founder_id, 'summary', summary, 'evidence', json(evidence_json), 'confidence', confidence, 'status', status, 'createdAt', created_at)) FROM learnings",
    )
    .await?;
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "exportedAt": created_at.to_rfc3339(),
        "includesSecrets": false,
        "founderProfiles": founder,
        "icpHypotheses": icp,
        "contentIdeas": ideas,
        "experiments": experiments,
        "learnings": learnings
    });
    tokio::fs::write(&path, serde_json::to_vec_pretty(&manifest)?).await?;
    Ok(DataArtifact {
        path: path.to_string_lossy().into_owned(),
        kind: "json_export".to_owned(),
        created_at: created_at.to_rfc3339(),
        includes_secrets: false,
    })
}

pub async fn backup_database(state: &AppState) -> AppResult<DataArtifact> {
    let root = state
        .database
        .path()?
        .parent()
        .ok_or_else(|| AppError::Internal("database has no parent directory".to_owned()))?;
    let directory = root.join("backups");
    tokio::fs::create_dir_all(&directory).await?;
    let created_at = Utc::now();
    let path = directory.join(format!(
        "tagline-{}.sqlite",
        created_at.format("%Y%m%d-%H%M%S")
    ));
    sqlx::query("VACUUM INTO ?")
        .bind(path.to_string_lossy().into_owned())
        .execute(state.database.pool())
        .await?;
    Ok(DataArtifact {
        path: path.to_string_lossy().into_owned(),
        kind: "sqlite_backup".to_owned(),
        created_at: created_at.to_rfc3339(),
        includes_secrets: false,
    })
}

pub async fn factory_reset(state: &AppState, confirmation: &str) -> AppResult<()> {
    if confirmation != "RESET LOCAL LAB" {
        return Err(AppError::Validation(
            "type RESET LOCAL LAB to confirm".to_owned(),
        ));
    }
    let accounts =
        crate::db::repositories::platform::PlatformRepository::new(state.database.pool().clone())
            .list()
            .await?;
    let mut transaction = state.database.pool().begin().await?;
    for table in [
        "job_attempts",
        "jobs",
        "messages",
        "conversations",
        "relationship_identities",
        "relationships",
        "metric_snapshots",
        "remote_content",
        "sync_cursors",
        "oauth_transactions",
        "connected_accounts",
        "approvals",
        "content_variants",
        "experiments",
        "content_ideas",
        "learnings",
        "icp_evidence",
        "icp_hypotheses",
        "voice_examples",
        "voice_profiles",
        "audit_events",
        "founder_profiles",
        "app_settings",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    for account in accounts {
        let _ = state.secrets.delete(&account.secret_ref);
    }
    Ok(())
}

async fn json_array(pool: &SqlitePool, query: &str) -> AppResult<serde_json::Value> {
    let row = sqlx::query(query).fetch_one(pool).await?;
    let value: String = row.try_get(0)?;
    Ok(serde_json::from_str(&value)?)
}

#[cfg(test)]
mod tests {
    use super::factory_reset;
    use crate::app_state::AppState;

    #[tokio::test]
    async fn reset_requires_exact_confirmation() {
        let state = AppState::for_tests().await.expect("state");
        assert!(factory_reset(&state, "reset").await.is_err());
        factory_reset(&state, "RESET LOCAL LAB")
            .await
            .expect("reset");
    }
}
