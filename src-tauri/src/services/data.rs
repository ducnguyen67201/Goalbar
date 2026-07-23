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
        "goalbar-{}.json",
        created_at.format("%Y%m%d-%H%M%S")
    ));
    let founder = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'name', name, 'productName', product_name, 'offer', offer, 'expertise', expertise, 'goals', json(goals_json), 'boundaries', json(boundaries_json), 'createdAt', created_at, 'updatedAt', updated_at)) FROM founder_profiles",
    )
    .await?;
    let icp = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'founderId', founder_id, 'version', version, 'parentId', parent_id, 'role', role, 'situation', situation, 'urgentProblem', urgent_problem, 'desiredOutcome', desired_outcome, 'confidence', confidence, 'status', status, 'createdAt', created_at)) FROM icp_hypotheses",
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
        "SELECT json_group_array(json_object('id', id, 'founderId', founder_id, 'growthActionId', growth_action_id, 'summary', summary, 'evidence', json(evidence_json), 'confidence', confidence, 'status', status, 'createdAt', created_at)) FROM learnings",
    )
    .await?;
    let ingestion_sources = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'platform', platform, 'sourceKind', source_kind, 'ownership', ownership, 'displayName', display_name, 'accountHandle', account_handle, 'metadata', json(metadata_json), 'createdAt', created_at)) FROM ingestion_sources",
    )
    .await?;
    let activity_items = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'sourceId', source_id, 'platform', platform, 'itemKind', item_kind, 'ownership', ownership, 'direction', direction, 'remoteId', remote_id, 'canonicalUrl', canonical_url, 'authorHandle', author_handle, 'counterpartyHandle', counterparty_handle, 'body', body, 'publishedAt', published_at, 'observedAt', observed_at, 'metadata', json(metadata_json))) FROM activity_items",
    )
    .await?;
    let ingestion_runs = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'sourceId', source_id, 'status', status, 'provider', provider, 'objective', objective, 'limits', json(limits_json), 'counts', json(counts_json), 'pauseReason', pause_reason, 'errorCode', error_code, 'startedAt', started_at, 'updatedAt', updated_at, 'completedAt', completed_at)) FROM ingestion_runs",
    )
    .await?;
    let growth_actions = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'founderId', founder_id, 'icpHypothesisId', icp_hypothesis_id, 'experimentId', experiment_id, 'kind', kind, 'platform', platform, 'title', title, 'rationale', rationale, 'targetUrl', target_url, 'exactPayload', exact_payload, 'payloadHash', payload_hash, 'revision', revision, 'hypothesis', hypothesis, 'successMetric', success_metric, 'evaluationWindowDays', evaluation_window_days, 'status', status, 'scheduledFor', scheduled_for, 'completedAt', completed_at, 'createdAt', created_at, 'updatedAt', updated_at)) FROM growth_actions",
    )
    .await?;
    let growth_action_executions = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'actionId', action_id, 'approvalId', approval_id, 'outcome', outcome, 'resultUrl', result_url, 'detail', detail, 'attemptedAt', attempted_at)) FROM growth_action_executions",
    )
    .await?;
    let growth_action_metrics = json_array(
        state.database.pool(),
        "SELECT json_group_array(json_object('id', id, 'actionId', action_id, 'metricName', metric_name, 'value', value, 'availability', availability, 'sourceDefinition', source_definition, 'notes', notes, 'observedAt', observed_at, 'collectedAt', collected_at)) FROM growth_action_metrics",
    )
    .await?;
    let manifest = serde_json::json!({
        "schemaVersion": 3,
        "exportedAt": created_at.to_rfc3339(),
        "includesSecrets": false,
        "founderProfiles": founder,
        "icpHypotheses": icp,
        "contentIdeas": ideas,
        "experiments": experiments,
        "learnings": learnings,
        "ingestionSources": ingestion_sources,
        "ingestionRuns": ingestion_runs,
        "activityItems": activity_items,
        "growthActions": growth_actions,
        "growthActionExecutions": growth_action_executions,
        "growthActionMetrics": growth_action_metrics
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
        "goalbar-{}.sqlite",
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
        "growth_action_metrics",
        "growth_action_executions",
        "growth_actions",
        "browser_checkpoints",
        "activity_items",
        "ingestion_runs",
        "ingestion_sources",
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
        sqlx::query("INSERT INTO ingestion_sources (id, platform, source_kind, ownership, display_name, source_fingerprint, metadata_json, created_at) VALUES (?, 'x', 'archive', 'own', 'fixture', 'fixture', '{}', ?)")
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(state.database.pool())
            .await
            .expect("seed ingestion source");
        factory_reset(&state, "RESET LOCAL LAB")
            .await
            .expect("reset");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ingestion_sources")
            .fetch_one(state.database.pool())
            .await
            .expect("source count");
        assert_eq!(count, 0);
    }
}
