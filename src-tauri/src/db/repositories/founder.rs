use chrono::{DateTime, Utc};
use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::domain::founder::{FounderProfile, FounderProfileInput, VoiceProfileInput};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct FounderRepository {
    pool: SqlitePool,
}

impl FounderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save(&self, input: FounderProfileInput) -> AppResult<FounderProfile> {
        let profile = FounderProfile::new(input)?;
        sqlx::query(
            "INSERT INTO founder_profiles (id, name, product_name, offer, expertise, goals_json, boundaries_json, onboarding_completed, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.input.name)
        .bind(&profile.input.product_name)
        .bind(&profile.input.offer)
        .bind(&profile.input.expertise)
        .bind(serde_json::to_string(&profile.input.goals)?)
        .bind(serde_json::to_string(&profile.input.boundaries)?)
        .bind(profile.created_at.to_rfc3339())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(profile)
    }

    pub async fn latest(&self) -> AppResult<Option<FounderProfile>> {
        let row = sqlx::query(
            "SELECT id, name, product_name, offer, expertise, goals_json, boundaries_json, onboarding_completed, created_at, updated_at FROM founder_profiles ORDER BY updated_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(FounderProfile {
                id: Uuid::parse_str(row.try_get("id")?)
                    .map_err(|error| AppError::Internal(error.to_string()))?,
                input: FounderProfileInput {
                    name: row.try_get("name")?,
                    product_name: row.try_get("product_name")?,
                    offer: row.try_get("offer")?,
                    expertise: row.try_get("expertise")?,
                    goals: serde_json::from_str(row.try_get("goals_json")?)?,
                    boundaries: serde_json::from_str(row.try_get("boundaries_json")?)?,
                },
                onboarding_completed: row.try_get::<i64, _>("onboarding_completed")? != 0,
                created_at: parse_time(row.try_get("created_at")?)?,
                updated_at: parse_time(row.try_get("updated_at")?)?,
            })
        })
        .transpose()
    }

    pub async fn save_voice(&self, founder_id: Uuid, input: VoiceProfileInput) -> AppResult<Uuid> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query("UPDATE voice_profiles SET active = 0 WHERE founder_id = ?")
            .bind(founder_id.to_string())
            .execute(&mut *transaction)
            .await?;
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO voice_profiles (id, founder_id, traits_json, do_rules_json, dont_rules_json, active, version, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 1, 1, ?, ?)")
            .bind(id.to_string())
            .bind(founder_id.to_string())
            .bind(serde_json::to_string(&input.traits)?)
            .bind(serde_json::to_string(&input.do_rules)?)
            .bind(serde_json::to_string(&input.dont_rules)?)
            .bind(&now)
            .bind(&now)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("INSERT INTO voice_examples (id, voice_profile_id, source, original_text, approved_text, created_at) VALUES (?, ?, 'onboarding', ?, ?, ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(id.to_string())
            .bind(&input.example)
            .bind(&input.example)
            .bind(&now)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(id)
    }
}

fn parse_time(value: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::FounderRepository;
    use crate::db::Database;
    use crate::domain::founder::FounderProfileInput;

    #[tokio::test]
    async fn saves_and_reads_founder() {
        let database = Database::in_memory().await.expect("database");
        let repository = FounderRepository::new(database.pool().clone());
        let saved = repository
            .save(FounderProfileInput {
                name: "Duc".to_owned(),
                product_name: "Lab".to_owned(),
                offer: "Growth system".to_owned(),
                expertise: "Product engineering".to_owned(),
                goals: vec!["Qualified conversations".to_owned()],
                boundaries: vec!["No spam".to_owned()],
            })
            .await
            .expect("profile");
        assert_eq!(
            repository
                .latest()
                .await
                .expect("latest")
                .expect("profile")
                .id,
            saved.id
        );
    }
}
