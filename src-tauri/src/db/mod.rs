pub mod migrations;
pub mod repositories;

use std::path::Path;
use std::str::FromStr as _;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
    path: Option<std::path::PathBuf>,
}

impl Database {
    pub async fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let url = format!("sqlite://{}", path.to_string_lossy());
        let options = SqliteConnectOptions::from_str(&url)
            .map_err(|error| AppError::Database(sqlx::Error::Configuration(Box::new(error))))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        migrations::run(&pool).await.map_err(|error| {
            AppError::Internal(format!(
                "database migration failed; the original file was preserved: {error}"
            ))
        })?;
        Ok(Self {
            pool,
            path: Some(path.to_path_buf()),
        })
    }

    pub async fn in_memory() -> AppResult<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(|error| AppError::Database(sqlx::Error::Configuration(Box::new(error))))?
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        migrations::run(&pool)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(Self { pool, path: None })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn path(&self) -> AppResult<&Path> {
        self.path.as_deref().ok_or_else(|| {
            AppError::Unsupported("in-memory databases cannot be backed up".to_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Database;

    #[tokio::test]
    async fn fresh_database_runs_all_migrations() {
        let database = Database::in_memory()
            .await
            .expect("database should migrate");
        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('founder_profiles', 'jobs', 'metric_snapshots')",
        )
        .fetch_one(database.pool())
        .await
        .expect("table count should load");
        assert_eq!(tables, 3);
    }
}
