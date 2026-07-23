use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Platform;
use super::icp::StoredIcpHypothesis;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GrowthActionKind {
    Research,
    Follow,
    Comment,
    Post,
}

impl GrowthActionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Research => "research",
            Self::Follow => "follow",
            Self::Comment => "comment",
            Self::Post => "post",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "research" => Ok(Self::Research),
            "follow" => Ok(Self::Follow),
            "comment" => Ok(Self::Comment),
            "post" => Ok(Self::Post),
            _ => Err(AppError::Validation(format!(
                "invalid growth action kind: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GrowthActionStatus {
    Proposed,
    Approved,
    Completed,
    Failed,
    Cancelled,
    Measured,
}

impl GrowthActionStatus {
    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "proposed" => Ok(Self::Proposed),
            "approved" => Ok(Self::Approved),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "measured" => Ok(Self::Measured),
            _ => Err(AppError::Validation(format!(
                "invalid growth action status: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProposeGrowthActionInput {
    pub icp_hypothesis_id: Option<Uuid>,
    pub experiment_id: Option<Uuid>,
    pub kind: GrowthActionKind,
    pub platform: Option<Platform>,
    pub title: String,
    pub rationale: String,
    pub target_url: Option<String>,
    pub exact_payload: String,
    pub hypothesis: String,
    pub success_metric: String,
    pub evaluation_window_days: u16,
    pub scheduled_for: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrowthAction {
    pub id: Uuid,
    pub founder_id: Uuid,
    pub icp_hypothesis_id: Option<Uuid>,
    pub experiment_id: Option<Uuid>,
    pub kind: GrowthActionKind,
    pub platform: Option<Platform>,
    pub title: String,
    pub rationale: String,
    pub target_url: Option<String>,
    pub exact_payload: String,
    pub payload_hash: String,
    pub revision: u32,
    pub hypothesis: String,
    pub success_metric: String,
    pub evaluation_window_days: u16,
    pub status: GrowthActionStatus,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub approval_id: Option<Uuid>,
    pub executions: Vec<GrowthActionExecution>,
    pub metrics: Vec<GrowthActionMetric>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOutcome {
    Succeeded,
    Failed,
}

impl ExecutionOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(AppError::Validation(format!(
                "invalid growth action outcome: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrowthActionExecution {
    pub id: Uuid,
    pub action_id: Uuid,
    pub approval_id: Uuid,
    pub outcome: ExecutionOutcome,
    pub result_url: Option<String>,
    pub detail: String,
    pub attempted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricAvailability {
    Available,
    Missing,
    Restricted,
    Delayed,
}

impl MetricAvailability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Missing => "missing",
            Self::Restricted => "restricted",
            Self::Delayed => "delayed",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "available" => Ok(Self::Available),
            "missing" => Ok(Self::Missing),
            "restricted" => Ok(Self::Restricted),
            "delayed" => Ok(Self::Delayed),
            _ => Err(AppError::Validation(format!(
                "invalid metric availability: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrowthActionMetric {
    pub id: Uuid,
    pub action_id: Uuid,
    pub metric_name: String,
    pub value: Option<f64>,
    pub availability: MetricAvailability,
    pub source_definition: String,
    pub notes: String,
    pub observed_at: DateTime<Utc>,
    pub collected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApproveGrowthActionInput {
    pub action_id: Uuid,
    pub exact_payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviseGrowthActionInput {
    pub action_id: Uuid,
    pub exact_payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordGrowthActionExecutionInput {
    pub action_id: Uuid,
    pub approval_id: Uuid,
    pub exact_payload: String,
    pub outcome: ExecutionOutcome,
    pub result_url: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordGrowthActionMetricInput {
    pub action_id: Uuid,
    pub metric_name: String,
    pub value: Option<f64>,
    pub availability: MetricAvailability,
    pub source_definition: String,
    pub notes: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordGrowthLearningInput {
    pub action_id: Uuid,
    pub observation: String,
    pub learning: String,
    pub counter_evidence: Vec<String>,
    pub confidence: f64,
    pub next_experiment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackedGrowthLearning {
    pub id: Uuid,
    pub growth_action_id: Option<Uuid>,
    pub summary: String,
    pub evidence: serde_json::Value,
    pub confidence: f64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrowthLoopTotals {
    pub proposed: u32,
    pub approved: u32,
    pub completed: u32,
    pub measured: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrowthLoopOverview {
    pub schema_version: u16,
    pub active_icp: Option<StoredIcpHypothesis>,
    pub actions: Vec<GrowthAction>,
    pub learnings: Vec<TrackedGrowthLearning>,
    pub totals: GrowthLoopTotals,
}
