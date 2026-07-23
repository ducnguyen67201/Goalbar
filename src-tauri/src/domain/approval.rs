use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::validation::payload_hash;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: Uuid,
    pub subject_type: String,
    pub subject_id: Uuid,
    pub payload_hash: String,
    pub idempotency_key: Uuid,
    pub approved_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub invalidated_at: Option<DateTime<Utc>>,
}

impl Approval {
    pub fn new(subject_type: impl Into<String>, subject_id: Uuid, payload: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject_type: subject_type.into(),
            subject_id,
            payload_hash: payload_hash(payload),
            idempotency_key: Uuid::new_v4(),
            approved_at: Utc::now(),
            consumed_at: None,
            invalidated_at: None,
        }
    }

    pub fn permits(&self, payload: &str) -> bool {
        self.consumed_at.is_none()
            && self.invalidated_at.is_none()
            && self.payload_hash == payload_hash(payload)
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::Approval;

    #[test]
    fn edits_invalidate_approval_payload() {
        let approval = Approval::new("reply", Uuid::new_v4(), "original");
        assert!(approval.permits("original"));
        assert!(!approval.permits("edited"));
    }
}
