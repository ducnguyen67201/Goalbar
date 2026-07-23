use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IcpHypothesisDraft {
    pub role: String,
    pub situation: String,
    pub urgent_problem: String,
    pub current_workaround: String,
    pub desired_outcome: String,
    pub objections: Vec<String>,
    pub language: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IcpHypotheses {
    pub hypotheses: Vec<IcpHypothesisDraft>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IcpStatus {
    Proposed,
    Active,
    Rejected,
    Archived,
}

impl IcpStatus {
    pub fn parse(value: &str) -> crate::error::AppResult<Self> {
        match value {
            "proposed" => Ok(Self::Proposed),
            "active" => Ok(Self::Active),
            "rejected" => Ok(Self::Rejected),
            "archived" => Ok(Self::Archived),
            _ => Err(crate::error::AppError::Validation(format!(
                "invalid ICP status: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StoredIcpHypothesis {
    pub id: Uuid,
    pub founder_id: Uuid,
    pub version: u32,
    pub parent_id: Option<Uuid>,
    #[serde(flatten)]
    pub draft: IcpHypothesisDraft,
    pub status: IcpStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDirection {
    Supports,
    Contradicts,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IcpEvidence {
    pub id: Uuid,
    pub hypothesis_id: Uuid,
    pub summary: String,
    pub direction: EvidenceDirection,
    pub weight: f64,
    pub accepted: bool,
}

pub fn revised_confidence(current: f64, evidence: &[IcpEvidence]) -> f64 {
    evidence
        .iter()
        .filter(|item| item.accepted)
        .fold(current, |score, item| {
            let delta = match item.direction {
                EvidenceDirection::Supports => item.weight * 0.12,
                EvidenceDirection::Contradicts => item.weight * -0.16,
                EvidenceDirection::Neutral => 0.0,
            };
            (score + delta).clamp(0.0, 1.0)
        })
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{EvidenceDirection, IcpEvidence, revised_confidence};

    #[test]
    fn confidence_uses_only_accepted_evidence() {
        let hypothesis_id = Uuid::new_v4();
        let evidence = vec![
            IcpEvidence {
                id: Uuid::new_v4(),
                hypothesis_id,
                summary: "Strong conversation".to_owned(),
                direction: EvidenceDirection::Supports,
                weight: 1.0,
                accepted: true,
            },
            IcpEvidence {
                id: Uuid::new_v4(),
                hypothesis_id,
                summary: "Unreviewed".to_owned(),
                direction: EvidenceDirection::Contradicts,
                weight: 1.0,
                accepted: false,
            },
        ];
        assert!((revised_confidence(0.5, &evidence) - 0.62).abs() < f64::EPSILON);
    }
}
