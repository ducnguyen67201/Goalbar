use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrowthInputs {
    pub attention_quality: Option<f64>,
    pub conversation_quality: Option<f64>,
    pub relationship_growth: Option<f64>,
    pub consistency: Option<f64>,
    pub learning_velocity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrowthScore {
    pub formula_version: u16,
    pub score: f64,
    pub confidence: f64,
    pub components: GrowthInputs,
    pub missing: Vec<String>,
}

pub fn calculate_growth_score(inputs: GrowthInputs) -> GrowthScore {
    let components = [
        ("attentionQuality", inputs.attention_quality, 0.15),
        ("conversationQuality", inputs.conversation_quality, 0.25),
        ("relationshipGrowth", inputs.relationship_growth, 0.25),
        ("consistency", inputs.consistency, 0.15),
        ("learningVelocity", inputs.learning_velocity, 0.20),
    ];
    let available_weight: f64 = components
        .iter()
        .filter_map(|(_, value, weight)| value.map(|_| *weight))
        .sum();
    let weighted: f64 = components
        .iter()
        .filter_map(|(_, value, weight)| value.map(|value| value.clamp(0.0, 100.0) * weight))
        .sum();
    let missing = components
        .iter()
        .filter(|(_, value, _)| value.is_none())
        .map(|(name, _, _)| (*name).to_owned())
        .collect();
    GrowthScore {
        formula_version: 1,
        score: if available_weight > 0.0 {
            weighted / available_weight
        } else {
            0.0
        },
        confidence: available_weight,
        components: inputs,
        missing,
    }
}

#[cfg(test)]
mod tests {
    use super::{GrowthInputs, calculate_growth_score};

    #[test]
    fn missing_metrics_reduce_confidence_not_score_by_zero_fill() {
        let score = calculate_growth_score(GrowthInputs {
            conversation_quality: Some(80.0),
            relationship_growth: Some(60.0),
            ..GrowthInputs::default()
        });
        assert_eq!(score.score, 70.0);
        assert_eq!(score.confidence, 0.5);
        assert_eq!(score.missing.len(), 3);
    }

    #[test]
    fn values_are_clamped() {
        let score = calculate_growth_score(GrowthInputs {
            attention_quality: Some(120.0),
            conversation_quality: Some(120.0),
            relationship_growth: Some(120.0),
            consistency: Some(120.0),
            learning_velocity: Some(120.0),
        });
        assert_eq!(score.score, 100.0);
    }
}
