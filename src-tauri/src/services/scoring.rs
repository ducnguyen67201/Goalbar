use crate::db::repositories::metrics::MetricsRepository;
use crate::domain::metrics::GrowthScore;
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct ScoringService {
    repository: MetricsRepository,
}

impl ScoringService {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            repository: MetricsRepository::new(pool),
        }
    }

    pub async fn current(&self) -> AppResult<GrowthScore> {
        self.repository.current_score().await
    }
}
