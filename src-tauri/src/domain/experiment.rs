use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Draft,
    Running,
    Measuring,
    Complete,
    Cancelled,
}

impl ExperimentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Running => "running",
            Self::Measuring => "measuring",
            Self::Complete => "complete",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Experiment {
    pub id: Uuid,
    pub hypothesis: String,
    pub success_metric: String,
    pub window_days: u16,
    pub status: ExperimentStatus,
}

#[cfg(test)]
mod tests {
    use super::ExperimentStatus;

    #[test]
    fn status_has_stable_storage_value() {
        assert_eq!(ExperimentStatus::Measuring.as_str(), "measuring");
    }
}
