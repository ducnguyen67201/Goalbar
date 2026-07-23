use chrono::{Duration, Utc};
use serde_json::Value;
use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::domain::job::JobStatus;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct JobRecord {
    pub id: Uuid,
    pub kind: String,
    pub status: JobStatus,
    pub payload: Value,
    pub attempts: u32,
    pub max_attempts: u32,
}

#[derive(Debug, Clone)]
pub struct JobRepository {
    pool: SqlitePool,
}

impl JobRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn enqueue(&self, kind: &str, payload: &Value) -> AppResult<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO jobs (id, kind, status, payload_json, created_at, updated_at) VALUES (?, ?, 'pending', ?, ?, ?)")
            .bind(id.to_string())
            .bind(kind)
            .bind(payload.to_string())
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        Ok(id)
    }

    pub async fn lease_next(&self) -> AppResult<Option<JobRecord>> {
        let mut transaction = self.pool.begin().await?;
        let now = Utc::now();
        let row = sqlx::query("SELECT id, kind, status, payload_json, attempts, max_attempts FROM jobs WHERE status = 'pending' AND (next_attempt_at IS NULL OR next_attempt_at <= ?) ORDER BY created_at LIMIT 1")
            .bind(now.to_rfc3339())
            .fetch_optional(&mut *transaction)
            .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let id: String = row.try_get("id")?;
        sqlx::query("UPDATE jobs SET status = 'running', attempts = attempts + 1, lease_expires_at = ?, updated_at = ? WHERE id = ? AND status = 'pending'")
            .bind((now + Duration::minutes(2)).to_rfc3339())
            .bind(now.to_rfc3339())
            .bind(&id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(Some(JobRecord {
            id: Uuid::parse_str(&id).map_err(|error| AppError::Internal(error.to_string()))?,
            kind: row.try_get("kind")?,
            status: JobStatus::Running,
            payload: serde_json::from_str(row.try_get("payload_json")?)?,
            attempts: row.try_get::<i64, _>("attempts")? as u32 + 1,
            max_attempts: row.try_get::<i64, _>("max_attempts")? as u32,
        }))
    }

    pub async fn finish(&self, id: Uuid, result: &Value) -> AppResult<()> {
        sqlx::query("UPDATE jobs SET status = 'completed', result_json = ?, lease_expires_at = NULL, updated_at = ? WHERE id = ? AND status = 'running'")
            .bind(result.to_string())
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn fail(&self, job: &JobRecord, error_code: &str) -> AppResult<()> {
        let retry = job.attempts < job.max_attempts;
        let next_attempt = retry.then(|| {
            let seconds = 2_i64.pow(job.attempts.min(8)) * 5;
            (Utc::now() + Duration::seconds(seconds)).to_rfc3339()
        });
        sqlx::query("UPDATE jobs SET status = ?, error_code = ?, lease_expires_at = NULL, next_attempt_at = ?, updated_at = ? WHERE id = ?")
            .bind(if retry { "pending" } else { "failed" })
            .bind(error_code)
            .bind(next_attempt)
            .bind(Utc::now().to_rfc3339())
            .bind(job.id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn cancel(&self, id: Uuid) -> AppResult<()> {
        let changed = sqlx::query("UPDATE jobs SET status = 'cancelled', lease_expires_at = NULL, updated_at = ? WHERE id = ? AND status IN ('pending', 'running')")
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected();
        if changed == 0 {
            return Err(AppError::NotFound(format!("cancellable job {id}")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::JobRepository;
    use crate::db::Database;

    #[tokio::test]
    async fn leases_each_job_once() {
        let database = Database::in_memory().await.expect("database");
        let repository = JobRepository::new(database.pool().clone());
        let id = repository
            .enqueue("sync", &json!({"platform": "x"}))
            .await
            .expect("enqueue");
        assert_eq!(
            repository
                .lease_next()
                .await
                .expect("lease")
                .expect("job")
                .id,
            id
        );
        assert!(
            repository
                .lease_next()
                .await
                .expect("second lease")
                .is_none()
        );
    }

    #[tokio::test]
    async fn failed_job_is_rescheduled_with_a_bound() {
        let database = Database::in_memory().await.expect("database");
        let repository = JobRepository::new(database.pool().clone());
        repository
            .enqueue("sync", &json!({}))
            .await
            .expect("enqueue");
        let job = repository.lease_next().await.expect("lease").expect("job");
        repository.fail(&job, "offline").await.expect("fail");
        let status: String = sqlx::query_scalar("SELECT status FROM jobs WHERE id = ?")
            .bind(job.id.to_string())
            .fetch_one(database.pool())
            .await
            .expect("status");
        assert_eq!(status, "pending");
    }
}
