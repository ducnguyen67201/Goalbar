use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WeeklyLearningDraft {
    pub observation: String,
    pub learning: String,
    pub counter_evidence: Vec<String>,
    pub confidence: f64,
    pub next_experiment: String,
}
