use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::validation::require_non_empty;

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

impl IcpHypothesisDraft {
    pub fn validate(mut self) -> AppResult<Self> {
        self.role = require_non_empty(&self.role, "ICP role", 240)?;
        self.situation = require_non_empty(&self.situation, "ICP situation", 2_000)?;
        self.urgent_problem = require_non_empty(&self.urgent_problem, "ICP urgent problem", 2_000)?;
        self.current_workaround =
            require_non_empty(&self.current_workaround, "ICP current workaround", 2_000)?;
        self.desired_outcome =
            require_non_empty(&self.desired_outcome, "ICP desired outcome", 2_000)?;
        self.objections = validated_terms(self.objections, "ICP objections")?;
        self.language = validated_terms(self.language, "ICP language")?;
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            return Err(AppError::Validation(
                "ICP confidence must be between 0 and 1".to_owned(),
            ));
        }
        Ok(self)
    }
}

fn validated_terms(values: Vec<String>, field: &str) -> AppResult<Vec<String>> {
    let values = values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.len() > 50 {
        return Err(AppError::Validation(format!(
            "{field} must contain at most 50 items"
        )));
    }
    if values.iter().any(|value| value.chars().count() > 500) {
        return Err(AppError::Validation(format!(
            "each {field} item must be at most 500 characters"
        )));
    }
    Ok(values)
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
