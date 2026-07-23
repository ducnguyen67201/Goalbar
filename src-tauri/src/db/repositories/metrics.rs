use sqlx::{Row as _, SqlitePool};

use crate::domain::metrics::{GrowthInputs, GrowthScore, calculate_growth_score};
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct MetricsRepository {
    pool: SqlitePool,
}

impl MetricsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn current_score(&self) -> AppResult<GrowthScore> {
        let rows = sqlx::query(
            "SELECT metric_name, AVG(value) AS average
             FROM (
                 SELECT metric_name, value, availability, observed_at FROM metric_snapshots
                 UNION ALL
                 SELECT metric_name, value, availability, observed_at FROM growth_action_metrics
             )
             WHERE availability = 'available'
               AND julianday(observed_at) >= julianday('now', '-28 days')
             GROUP BY metric_name",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut inputs = GrowthInputs::default();
        for row in rows {
            let value: Option<f64> = row.try_get("average")?;
            match row.try_get::<&str, _>("metric_name")? {
                "attention_quality" => inputs.attention_quality = value,
                "conversation_quality" => inputs.conversation_quality = value,
                "relationship_growth" => inputs.relationship_growth = value,
                "consistency" => inputs.consistency = value,
                "learning_velocity" => inputs.learning_velocity = value,
                _ => {}
            }
        }
        Ok(calculate_growth_score(inputs))
    }
}
