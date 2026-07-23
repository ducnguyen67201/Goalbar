use uuid::Uuid;

use crate::db::repositories::founder::FounderRepository;
use crate::domain::founder::{FounderProfile, FounderProfileInput, VoiceProfileInput};
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct OnboardingService {
    founders: FounderRepository,
}

impl OnboardingService {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            founders: FounderRepository::new(pool),
        }
    }

    pub async fn save_founder(&self, input: FounderProfileInput) -> AppResult<FounderProfile> {
        self.founders.save(input).await
    }

    pub async fn save_voice(&self, founder_id: Uuid, input: VoiceProfileInput) -> AppResult<Uuid> {
        self.founders.save_voice(founder_id, input).await
    }
}
