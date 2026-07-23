use chrono::{DateTime, Utc};
use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::domain::approval::Approval;
use crate::domain::growth_loop::{
    GrowthAction, GrowthActionExecution, GrowthActionMetric, GrowthActionStatus,
    GrowthLoopOverview, GrowthLoopTotals, MetricAvailability, ProposeGrowthActionInput,
    RecordGrowthActionExecutionInput, RecordGrowthActionMetricInput, RecordGrowthLearningInput,
    TrackedGrowthLearning,
};
use crate::domain::icp::IcpStatus;
use crate::error::{AppError, AppResult};
use crate::validation::{allowlisted_external_url, payload_hash, require_non_empty};

use super::icp::IcpRepository;

#[derive(Debug, Clone)]
pub struct GrowthLoopRepository {
    pool: SqlitePool,
}

impl GrowthLoopRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn overview(&self, founder_id: Uuid) -> AppResult<GrowthLoopOverview> {
        let active_icp = IcpRepository::new(self.pool.clone())
            .list_for_founder(founder_id)
            .await?
            .into_iter()
            .find(|hypothesis| hypothesis.status == IcpStatus::Active);
        let actions = self.list_actions(founder_id).await?;
        let learnings = self.list_learnings(founder_id).await?;
        let totals = actions
            .iter()
            .fold(GrowthLoopTotals::default(), |mut totals, action| {
                match action.status {
                    GrowthActionStatus::Proposed => totals.proposed += 1,
                    GrowthActionStatus::Approved => totals.approved += 1,
                    GrowthActionStatus::Completed => totals.completed += 1,
                    GrowthActionStatus::Measured => totals.measured += 1,
                    GrowthActionStatus::Failed | GrowthActionStatus::Cancelled => {}
                }
                totals
            });
        Ok(GrowthLoopOverview {
            schema_version: 1,
            active_icp,
            actions,
            learnings,
            totals,
        })
    }

    pub async fn propose(
        &self,
        founder_id: Uuid,
        input: ProposeGrowthActionInput,
    ) -> AppResult<GrowthAction> {
        let title = require_non_empty(&input.title, "title", 200)?;
        let rationale = require_non_empty(&input.rationale, "rationale", 2_000)?;
        let exact_payload = require_non_empty(&input.exact_payload, "exact payload", 40_000)?;
        let hypothesis = require_non_empty(&input.hypothesis, "hypothesis", 2_000)?;
        let success_metric = require_non_empty(&input.success_metric, "success metric", 1_000)?;
        if !(1..=365).contains(&input.evaluation_window_days) {
            return Err(AppError::Validation(
                "evaluationWindowDays must be between 1 and 365".to_owned(),
            ));
        }
        let target_url = validate_optional_url(input.target_url.as_deref())?;
        let scheduled_for = parse_optional_time(input.scheduled_for.as_deref(), "scheduledFor")?;
        if let Some(hypothesis_id) = input.icp_hypothesis_id {
            self.ensure_icp_owner(founder_id, hypothesis_id).await?;
        }
        if let Some(experiment_id) = input.experiment_id {
            self.ensure_experiment_owner(founder_id, experiment_id)
                .await?;
        }

        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO growth_actions (id, founder_id, icp_hypothesis_id, experiment_id, kind, platform, title, rationale, target_url, exact_payload, payload_hash, revision, hypothesis, success_metric, evaluation_window_days, status, scheduled_for, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, 'proposed', ?, ?, ?)")
            .bind(id.to_string())
            .bind(founder_id.to_string())
            .bind(input.icp_hypothesis_id.map(|value| value.to_string()))
            .bind(input.experiment_id.map(|value| value.to_string()))
            .bind(input.kind.as_str())
            .bind(input.platform.map(|value| value.as_str()))
            .bind(title)
            .bind(rationale)
            .bind(target_url)
            .bind(&exact_payload)
            .bind(payload_hash(&exact_payload))
            .bind(hypothesis)
            .bind(success_metric)
            .bind(i64::from(input.evaluation_window_days))
            .bind(scheduled_for.map(|value| value.to_rfc3339()))
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        self.action(founder_id, id).await
    }

    pub async fn revise(
        &self,
        founder_id: Uuid,
        action_id: Uuid,
        exact_payload: &str,
    ) -> AppResult<GrowthAction> {
        let exact_payload = require_non_empty(exact_payload, "exact payload", 40_000)?;
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool.begin().await?;
        let updated = sqlx::query("UPDATE growth_actions SET exact_payload = ?, payload_hash = ?, revision = revision + 1, status = 'proposed', completed_at = NULL, updated_at = ? WHERE id = ? AND founder_id = ? AND status IN ('proposed', 'approved')")
            .bind(&exact_payload)
            .bind(payload_hash(&exact_payload))
            .bind(&now)
            .bind(action_id.to_string())
            .bind(founder_id.to_string())
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        if updated != 1 {
            return Err(AppError::Validation(
                "only proposed or approved actions can be revised".to_owned(),
            ));
        }
        sqlx::query("UPDATE approvals SET invalidated_at = ? WHERE subject_type = 'growth_action' AND subject_id = ? AND consumed_at IS NULL AND invalidated_at IS NULL")
            .bind(&now)
            .bind(action_id.to_string())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        self.action(founder_id, action_id).await
    }

    pub async fn approve(
        &self,
        founder_id: Uuid,
        action_id: Uuid,
        exact_payload: &str,
    ) -> AppResult<Approval> {
        let action = self.action(founder_id, action_id).await?;
        if action.status != GrowthActionStatus::Proposed || action.exact_payload != exact_payload {
            return Err(AppError::Validation(
                "action changed or is not awaiting approval".to_owned(),
            ));
        }
        let approval = Approval::new("growth_action", action_id, exact_payload);
        let mut transaction = self.pool.begin().await?;
        let updated = sqlx::query("UPDATE growth_actions SET status = 'approved', updated_at = ? WHERE id = ? AND founder_id = ? AND status = 'proposed' AND payload_hash = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(action_id.to_string())
            .bind(founder_id.to_string())
            .bind(&approval.payload_hash)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        if updated != 1 {
            return Err(AppError::Validation(
                "action changed while approval was being recorded".to_owned(),
            ));
        }
        sqlx::query("INSERT INTO approvals (id, subject_type, subject_id, payload_hash, idempotency_key, approved_at) VALUES (?, 'growth_action', ?, ?, ?, ?)")
            .bind(approval.id.to_string())
            .bind(action_id.to_string())
            .bind(&approval.payload_hash)
            .bind(approval.idempotency_key.to_string())
            .bind(approval.approved_at.to_rfc3339())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(approval)
    }

    pub async fn record_execution(
        &self,
        founder_id: Uuid,
        input: RecordGrowthActionExecutionInput,
    ) -> AppResult<GrowthAction> {
        let action = self.action(founder_id, input.action_id).await?;
        if action.status != GrowthActionStatus::Approved
            || action.exact_payload != input.exact_payload
        {
            return Err(AppError::Validation(
                "execution requires the currently approved exact payload".to_owned(),
            ));
        }
        let detail = require_non_empty(&input.detail, "execution detail", 2_000)?;
        let result_url = validate_optional_url(input.result_url.as_deref())?;
        let approval = self.approval(input.approval_id, input.action_id).await?;
        if !approval.permits(&input.exact_payload) {
            return Err(AppError::Validation(
                "approval is invalid, consumed, or belongs to another revision".to_owned(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let execution_id = Uuid::new_v4();
        let next_status = match input.outcome {
            crate::domain::growth_loop::ExecutionOutcome::Succeeded => "completed",
            crate::domain::growth_loop::ExecutionOutcome::Failed => "failed",
        };
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT INTO growth_action_executions (id, action_id, approval_id, outcome, result_url, detail, attempted_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(execution_id.to_string())
            .bind(input.action_id.to_string())
            .bind(input.approval_id.to_string())
            .bind(input.outcome.as_str())
            .bind(result_url)
            .bind(detail)
            .bind(&now)
            .execute(&mut *transaction)
            .await?;
        let consumed = sqlx::query("UPDATE approvals SET consumed_at = ? WHERE id = ? AND consumed_at IS NULL AND invalidated_at IS NULL")
            .bind(&now)
            .bind(input.approval_id.to_string())
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        if consumed != 1 {
            return Err(AppError::Validation(
                "approval was already consumed or invalidated".to_owned(),
            ));
        }
        sqlx::query("UPDATE growth_actions SET status = ?, completed_at = CASE WHEN ? = 'completed' THEN ? ELSE NULL END, updated_at = ? WHERE id = ? AND founder_id = ?")
            .bind(next_status)
            .bind(next_status)
            .bind(&now)
            .bind(&now)
            .bind(input.action_id.to_string())
            .bind(founder_id.to_string())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        self.action(founder_id, input.action_id).await
    }

    pub async fn record_metric(
        &self,
        founder_id: Uuid,
        input: RecordGrowthActionMetricInput,
    ) -> AppResult<GrowthAction> {
        let action = self.action(founder_id, input.action_id).await?;
        if !matches!(
            action.status,
            GrowthActionStatus::Completed | GrowthActionStatus::Measured
        ) {
            return Err(AppError::Validation(
                "metrics can only be recorded after a completed action".to_owned(),
            ));
        }
        let metric_name = normalized_metric_name(&input.metric_name)?;
        let source_definition =
            require_non_empty(&input.source_definition, "source definition", 1_000)?;
        let notes = input.notes.trim().chars().take(2_000).collect::<String>();
        let observed_at = parse_time(&input.observed_at, "observedAt")?;
        if input.availability == MetricAvailability::Available {
            let value = input.value.ok_or_else(|| {
                AppError::Validation("available metrics require a value".to_owned())
            })?;
            if !value.is_finite() || value < 0.0 {
                return Err(AppError::Validation(
                    "metric value must be a finite non-negative number".to_owned(),
                ));
            }
        } else if input.value.is_some() {
            return Err(AppError::Validation(
                "unavailable metrics cannot include a value".to_owned(),
            ));
        }
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT INTO growth_action_metrics (id, action_id, metric_name, value, availability, source_definition, notes, observed_at, collected_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(id.to_string())
            .bind(input.action_id.to_string())
            .bind(metric_name)
            .bind(input.value)
            .bind(input.availability.as_str())
            .bind(source_definition)
            .bind(notes)
            .bind(observed_at.to_rfc3339())
            .bind(&now)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE growth_actions SET status = 'measured', updated_at = ? WHERE id = ? AND founder_id = ?")
            .bind(&now)
            .bind(input.action_id.to_string())
            .bind(founder_id.to_string())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        self.action(founder_id, input.action_id).await
    }

    pub async fn record_learning(
        &self,
        founder_id: Uuid,
        input: RecordGrowthLearningInput,
    ) -> AppResult<TrackedGrowthLearning> {
        let action = self.action(founder_id, input.action_id).await?;
        if action.status != GrowthActionStatus::Measured {
            return Err(AppError::Validation(
                "record at least one metric before accepting a learning".to_owned(),
            ));
        }
        let observation = require_non_empty(&input.observation, "observation", 4_000)?;
        let learning = require_non_empty(&input.learning, "learning", 4_000)?;
        let next_experiment = require_non_empty(&input.next_experiment, "next experiment", 2_000)?;
        if !input.confidence.is_finite() || !(0.0..=1.0).contains(&input.confidence) {
            return Err(AppError::Validation(
                "learning confidence must be between 0 and 1".to_owned(),
            ));
        }
        let counter_evidence = input
            .counter_evidence
            .into_iter()
            .map(|value| value.trim().chars().take(1_000).collect::<String>())
            .filter(|value| !value.is_empty())
            .take(20)
            .collect::<Vec<_>>();
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO learnings (id, founder_id, growth_action_id, summary, evidence_json, confidence, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 'accepted', ?, ?)")
            .bind(id.to_string())
            .bind(founder_id.to_string())
            .bind(input.action_id.to_string())
            .bind(learning)
            .bind(
                serde_json::json!({
                    "observation": observation,
                    "counterEvidence": counter_evidence,
                    "nextExperiment": next_experiment,
                })
                .to_string(),
            )
            .bind(input.confidence)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        self.learning(id).await
    }

    async fn list_actions(&self, founder_id: Uuid) -> AppResult<Vec<GrowthAction>> {
        let rows = sqlx::query(ACTION_SELECT)
            .bind(founder_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        let mut actions = Vec::with_capacity(rows.len());
        for row in rows {
            let mut action = row_to_action(&row)?;
            action.executions = self.executions(action.id).await?;
            action.metrics = self.metrics(action.id).await?;
            actions.push(action);
        }
        Ok(actions)
    }

    async fn action(&self, founder_id: Uuid, action_id: Uuid) -> AppResult<GrowthAction> {
        let row = sqlx::query(ACTION_BY_ID_SELECT)
            .bind(founder_id.to_string())
            .bind(action_id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("growth action {action_id}")))?;
        let mut action = row_to_action(&row)?;
        action.executions = self.executions(action.id).await?;
        action.metrics = self.metrics(action.id).await?;
        Ok(action)
    }

    async fn executions(&self, action_id: Uuid) -> AppResult<Vec<GrowthActionExecution>> {
        let rows = sqlx::query("SELECT id, action_id, approval_id, outcome, result_url, detail, attempted_at FROM growth_action_executions WHERE action_id = ? ORDER BY attempted_at DESC")
            .bind(action_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_execution).collect()
    }

    async fn metrics(&self, action_id: Uuid) -> AppResult<Vec<GrowthActionMetric>> {
        let rows = sqlx::query("SELECT id, action_id, metric_name, value, availability, source_definition, notes, observed_at, collected_at FROM growth_action_metrics WHERE action_id = ? ORDER BY observed_at DESC")
            .bind(action_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_metric).collect()
    }

    async fn list_learnings(&self, founder_id: Uuid) -> AppResult<Vec<TrackedGrowthLearning>> {
        let rows = sqlx::query("SELECT id, growth_action_id, summary, evidence_json, confidence, status, created_at FROM learnings WHERE founder_id = ? ORDER BY created_at DESC LIMIT 20")
            .bind(founder_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_learning).collect()
    }

    async fn learning(&self, learning_id: Uuid) -> AppResult<TrackedGrowthLearning> {
        let row = sqlx::query("SELECT id, growth_action_id, summary, evidence_json, confidence, status, created_at FROM learnings WHERE id = ?")
            .bind(learning_id.to_string())
            .fetch_one(&self.pool)
            .await?;
        row_to_learning(&row)
    }

    async fn approval(&self, approval_id: Uuid, action_id: Uuid) -> AppResult<Approval> {
        let row = sqlx::query("SELECT id, subject_type, subject_id, payload_hash, idempotency_key, approved_at, consumed_at, invalidated_at FROM approvals WHERE id = ? AND subject_type = 'growth_action' AND subject_id = ?")
            .bind(approval_id.to_string())
            .bind(action_id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("growth action approval {approval_id}")))?;
        Ok(Approval {
            id: parse_uuid(row.try_get("id")?)?,
            subject_type: row.try_get("subject_type")?,
            subject_id: parse_uuid(row.try_get("subject_id")?)?,
            payload_hash: row.try_get("payload_hash")?,
            idempotency_key: parse_uuid(row.try_get("idempotency_key")?)?,
            approved_at: parse_time(row.try_get("approved_at")?, "approvedAt")?,
            consumed_at: parse_optional_db_time(row.try_get("consumed_at")?)?,
            invalidated_at: parse_optional_db_time(row.try_get("invalidated_at")?)?,
        })
    }

    async fn ensure_icp_owner(&self, founder_id: Uuid, hypothesis_id: Uuid) -> AppResult<()> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM icp_hypotheses WHERE id = ? AND founder_id = ?",
        )
        .bind(hypothesis_id.to_string())
        .bind(founder_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        if exists != 1 {
            return Err(AppError::Validation(
                "ICP hypothesis does not belong to this founder".to_owned(),
            ));
        }
        Ok(())
    }

    async fn ensure_experiment_owner(
        &self,
        founder_id: Uuid,
        experiment_id: Uuid,
    ) -> AppResult<()> {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM experiments e JOIN content_ideas i ON i.id = e.idea_id WHERE e.id = ? AND i.founder_id = ?")
            .bind(experiment_id.to_string())
            .bind(founder_id.to_string())
            .fetch_one(&self.pool)
            .await?;
        if exists != 1 {
            return Err(AppError::Validation(
                "experiment does not belong to this founder".to_owned(),
            ));
        }
        Ok(())
    }
}

const ACTION_SELECT: &str = "SELECT ga.id, ga.founder_id, ga.icp_hypothesis_id, ga.experiment_id, ga.kind, ga.platform, ga.title, ga.rationale, ga.target_url, ga.exact_payload, ga.payload_hash, ga.revision, ga.hypothesis, ga.success_metric, ga.evaluation_window_days, ga.status, ga.scheduled_for, ga.completed_at, ga.created_at, ga.updated_at, (SELECT a.id FROM approvals a WHERE a.subject_type = 'growth_action' AND a.subject_id = ga.id AND a.consumed_at IS NULL AND a.invalidated_at IS NULL ORDER BY a.approved_at DESC LIMIT 1) AS approval_id FROM growth_actions ga WHERE ga.founder_id = ? ORDER BY CASE ga.status WHEN 'approved' THEN 0 WHEN 'proposed' THEN 1 WHEN 'completed' THEN 2 WHEN 'measured' THEN 3 ELSE 4 END, COALESCE(ga.scheduled_for, ga.created_at), ga.created_at DESC LIMIT 100";
const ACTION_BY_ID_SELECT: &str = "SELECT ga.id, ga.founder_id, ga.icp_hypothesis_id, ga.experiment_id, ga.kind, ga.platform, ga.title, ga.rationale, ga.target_url, ga.exact_payload, ga.payload_hash, ga.revision, ga.hypothesis, ga.success_metric, ga.evaluation_window_days, ga.status, ga.scheduled_for, ga.completed_at, ga.created_at, ga.updated_at, (SELECT a.id FROM approvals a WHERE a.subject_type = 'growth_action' AND a.subject_id = ga.id AND a.consumed_at IS NULL AND a.invalidated_at IS NULL ORDER BY a.approved_at DESC LIMIT 1) AS approval_id FROM growth_actions ga WHERE ga.founder_id = ? AND ga.id = ?";

fn row_to_action(row: &sqlx::sqlite::SqliteRow) -> AppResult<GrowthAction> {
    let platform = row
        .try_get::<Option<&str>, _>("platform")?
        .map(crate::domain::Platform::parse)
        .transpose()?;
    Ok(GrowthAction {
        id: parse_uuid(row.try_get("id")?)?,
        founder_id: parse_uuid(row.try_get("founder_id")?)?,
        icp_hypothesis_id: parse_optional_uuid(row.try_get("icp_hypothesis_id")?)?,
        experiment_id: parse_optional_uuid(row.try_get("experiment_id")?)?,
        kind: crate::domain::growth_loop::GrowthActionKind::parse(row.try_get("kind")?)?,
        platform,
        title: row.try_get("title")?,
        rationale: row.try_get("rationale")?,
        target_url: row.try_get("target_url")?,
        exact_payload: row.try_get("exact_payload")?,
        payload_hash: row.try_get("payload_hash")?,
        revision: checked_u32(row.try_get("revision")?, "growth action revision")?,
        hypothesis: row.try_get("hypothesis")?,
        success_metric: row.try_get("success_metric")?,
        evaluation_window_days: checked_u16(
            row.try_get("evaluation_window_days")?,
            "evaluation window",
        )?,
        status: GrowthActionStatus::parse(row.try_get("status")?)?,
        scheduled_for: parse_optional_db_time(row.try_get("scheduled_for")?)?,
        completed_at: parse_optional_db_time(row.try_get("completed_at")?)?,
        created_at: parse_time(row.try_get("created_at")?, "createdAt")?,
        updated_at: parse_time(row.try_get("updated_at")?, "updatedAt")?,
        approval_id: parse_optional_uuid(row.try_get("approval_id")?)?,
        executions: Vec::new(),
        metrics: Vec::new(),
    })
}

fn row_to_execution(row: &sqlx::sqlite::SqliteRow) -> AppResult<GrowthActionExecution> {
    Ok(GrowthActionExecution {
        id: parse_uuid(row.try_get("id")?)?,
        action_id: parse_uuid(row.try_get("action_id")?)?,
        approval_id: parse_uuid(row.try_get("approval_id")?)?,
        outcome: crate::domain::growth_loop::ExecutionOutcome::parse(row.try_get("outcome")?)?,
        result_url: row.try_get("result_url")?,
        detail: row.try_get("detail")?,
        attempted_at: parse_time(row.try_get("attempted_at")?, "attemptedAt")?,
    })
}

fn row_to_metric(row: &sqlx::sqlite::SqliteRow) -> AppResult<GrowthActionMetric> {
    Ok(GrowthActionMetric {
        id: parse_uuid(row.try_get("id")?)?,
        action_id: parse_uuid(row.try_get("action_id")?)?,
        metric_name: row.try_get("metric_name")?,
        value: row.try_get("value")?,
        availability: MetricAvailability::parse(row.try_get("availability")?)?,
        source_definition: row.try_get("source_definition")?,
        notes: row.try_get("notes")?,
        observed_at: parse_time(row.try_get("observed_at")?, "observedAt")?,
        collected_at: parse_time(row.try_get("collected_at")?, "collectedAt")?,
    })
}

fn row_to_learning(row: &sqlx::sqlite::SqliteRow) -> AppResult<TrackedGrowthLearning> {
    Ok(TrackedGrowthLearning {
        id: parse_uuid(row.try_get("id")?)?,
        growth_action_id: parse_optional_uuid(row.try_get("growth_action_id")?)?,
        summary: row.try_get("summary")?,
        evidence: serde_json::from_str(row.try_get("evidence_json")?)?,
        confidence: row.try_get("confidence")?,
        status: row.try_get("status")?,
        created_at: parse_time(row.try_get("created_at")?, "createdAt")?,
    })
}

fn validate_optional_url(value: Option<&str>) -> AppResult<Option<String>> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            allowlisted_external_url(value)?;
            Ok(value.to_owned())
        })
        .transpose()
}

fn parse_optional_time(value: Option<&str>, field: &str) -> AppResult<Option<DateTime<Utc>>> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| parse_time(value, field))
        .transpose()
}

fn parse_time(value: &str, field: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::Validation(format!("{field} must be an RFC 3339 timestamp")))
}

fn parse_optional_db_time(value: Option<&str>) -> AppResult<Option<DateTime<Utc>>> {
    value
        .map(|value| parse_time(value, "stored timestamp"))
        .transpose()
        .map_err(|error| AppError::Internal(error.to_string()))
}

fn parse_uuid(value: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value).map_err(|error| AppError::Internal(error.to_string()))
}

fn parse_optional_uuid(value: Option<&str>) -> AppResult<Option<Uuid>> {
    value.map(parse_uuid).transpose()
}

fn checked_u32(value: i64, field: &str) -> AppResult<u32> {
    u32::try_from(value).map_err(|_| AppError::Internal(format!("{field} is outside u32")))
}

fn checked_u16(value: i64, field: &str) -> AppResult<u16> {
    u16::try_from(value).map_err(|_| AppError::Internal(format!("{field} is outside u16")))
}

fn normalized_metric_name(value: &str) -> AppResult<String> {
    let value = require_non_empty(value, "metric name", 100)?;
    if value.chars().any(|character| {
        !(character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_')
    }) {
        return Err(AppError::Validation(
            "metric name must use lowercase letters, numbers, and underscores".to_owned(),
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::db::Database;
    use crate::db::repositories::founder::FounderRepository;
    use crate::db::repositories::icp::IcpRepository;
    use crate::domain::Platform;
    use crate::domain::founder::FounderProfileInput;
    use crate::domain::growth_loop::{
        ApproveGrowthActionInput, ExecutionOutcome, GrowthActionKind, GrowthActionStatus,
        MetricAvailability, ProposeGrowthActionInput, RecordGrowthActionExecutionInput,
        RecordGrowthActionMetricInput, RecordGrowthLearningInput,
    };
    use crate::domain::icp::IcpHypothesisDraft;

    use super::GrowthLoopRepository;

    async fn seeded() -> (
        crate::domain::founder::FounderProfile,
        GrowthLoopRepository,
        uuid::Uuid,
    ) {
        let database = Database::in_memory().await.expect("database");
        let founder = FounderRepository::new(database.pool().clone())
            .save(FounderProfileInput {
                name: "Duc".to_owned(),
                product_name: "Goalbar".to_owned(),
                website_url: None,
                offer: "Controlled founder growth".to_owned(),
                ideal_customer: "Technical solo founders".to_owned(),
                expertise: "Local-first products".to_owned(),
                goals: vec!["Qualified conversations".to_owned()],
                boundaries: vec!["No spam".to_owned()],
            })
            .await
            .expect("founder");
        let icp = IcpRepository::new(database.pool().clone());
        let hypothesis_id = icp
            .save_hypothesis(
                founder.id,
                IcpHypothesisDraft {
                    role: "Technical solo founder".to_owned(),
                    situation: "Building in public".to_owned(),
                    urgent_problem: "Unstructured growth".to_owned(),
                    current_workaround: "Posting without tracking".to_owned(),
                    desired_outcome: "A sustainable learning loop".to_owned(),
                    objections: vec!["Automation spam".to_owned()],
                    language: vec!["controlled growth".to_owned()],
                    confidence: 0.6,
                },
            )
            .await
            .expect("hypothesis");
        icp.accept(founder.id, hypothesis_id)
            .await
            .expect("active ICP");
        (
            founder,
            GrowthLoopRepository::new(database.pool().clone()),
            hypothesis_id,
        )
    }

    #[tokio::test]
    async fn exact_approval_drives_action_measurement_and_learning() {
        let (founder, repository, hypothesis_id) = seeded().await;
        let action = repository
            .propose(
                founder.id,
                ProposeGrowthActionInput {
                    icp_hypothesis_id: Some(hypothesis_id),
                    experiment_id: None,
                    kind: GrowthActionKind::Comment,
                    platform: Some(Platform::X),
                    title: "Join one relevant founder conversation".to_owned(),
                    rationale: "The author matches the active ICP.".to_owned(),
                    target_url: Some("https://x.com/founder/status/1".to_owned()),
                    exact_payload: "Useful distinction: consistency compounds only when the feedback loop is explicit.".to_owned(),
                    hypothesis: "Specific comments create qualified replies.".to_owned(),
                    success_metric: "One qualified reply within seven days.".to_owned(),
                    evaluation_window_days: 7,
                    scheduled_for: None,
                },
            )
            .await
            .expect("action");
        assert_eq!(action.status, GrowthActionStatus::Proposed);
        assert!(
            repository
                .approve(founder.id, action.id, "changed")
                .await
                .is_err()
        );
        let approval = repository
            .approve(founder.id, action.id, &action.exact_payload)
            .await
            .expect("approval");
        let completed = repository
            .record_execution(
                founder.id,
                RecordGrowthActionExecutionInput {
                    action_id: action.id,
                    approval_id: approval.id,
                    exact_payload: action.exact_payload.clone(),
                    outcome: ExecutionOutcome::Succeeded,
                    result_url: Some("https://x.com/founder/status/1".to_owned()),
                    detail: "Founder confirmed the comment was submitted.".to_owned(),
                },
            )
            .await
            .expect("execution");
        assert_eq!(completed.status, GrowthActionStatus::Completed);
        let measured = repository
            .record_metric(
                founder.id,
                RecordGrowthActionMetricInput {
                    action_id: action.id,
                    metric_name: "qualified_replies".to_owned(),
                    value: Some(1.0),
                    availability: MetricAvailability::Available,
                    source_definition: "Replies manually verified as active ICP.".to_owned(),
                    notes: "One technical founder asked a follow-up.".to_owned(),
                    observed_at: Utc::now().to_rfc3339(),
                },
            )
            .await
            .expect("metric");
        assert_eq!(measured.status, GrowthActionStatus::Measured);
        repository
            .record_learning(
                founder.id,
                RecordGrowthLearningInput {
                    action_id: action.id,
                    observation: "A concrete distinction earned a follow-up.".to_owned(),
                    learning: "Specific operator insights outperform generic agreement.".to_owned(),
                    counter_evidence: vec![],
                    confidence: 0.55,
                    next_experiment: "Test the pattern in three more conversations.".to_owned(),
                },
            )
            .await
            .expect("learning");
        let overview = repository.overview(founder.id).await.expect("overview");
        assert_eq!(overview.totals.measured, 1);
        assert_eq!(overview.learnings.len(), 1);
    }

    #[tokio::test]
    async fn revision_invalidates_the_previous_approval() {
        let (founder, repository, hypothesis_id) = seeded().await;
        let action = repository
            .propose(
                founder.id,
                ProposeGrowthActionInput {
                    icp_hypothesis_id: Some(hypothesis_id),
                    experiment_id: None,
                    kind: GrowthActionKind::Post,
                    platform: Some(Platform::Linkedin),
                    title: "Publish founder learning".to_owned(),
                    rationale: "Tests the current ICP language.".to_owned(),
                    target_url: None,
                    exact_payload: "Original".to_owned(),
                    hypothesis: "Clear learning attracts the active ICP.".to_owned(),
                    success_metric: "Two qualified conversations.".to_owned(),
                    evaluation_window_days: 7,
                    scheduled_for: None,
                },
            )
            .await
            .expect("action");
        let approval = repository
            .approve(founder.id, action.id, "Original")
            .await
            .expect("approval");
        let revised = repository
            .revise(founder.id, action.id, "Revised")
            .await
            .expect("revision");
        assert_eq!(revised.revision, 2);
        assert_eq!(revised.status, GrowthActionStatus::Proposed);
        assert!(
            repository
                .record_execution(
                    founder.id,
                    RecordGrowthActionExecutionInput {
                        action_id: action.id,
                        approval_id: approval.id,
                        exact_payload: "Original".to_owned(),
                        outcome: ExecutionOutcome::Succeeded,
                        result_url: None,
                        detail: "Should not be accepted.".to_owned(),
                    },
                )
                .await
                .is_err()
        );
    }

    #[test]
    fn command_input_remains_exact_revision_scoped() {
        let input = ApproveGrowthActionInput {
            action_id: uuid::Uuid::new_v4(),
            exact_payload: "Exact content".to_owned(),
        };
        assert_eq!(input.exact_payload, "Exact content");
    }
}
