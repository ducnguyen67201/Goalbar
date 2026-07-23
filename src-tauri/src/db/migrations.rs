use base64::Engine as _;
use sha2::{Digest as _, Sha256};
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};

struct EmbeddedMigration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[EmbeddedMigration] = &[
    EmbeddedMigration {
        version: 1,
        name: "core",
        sql: include_str!("../../migrations/0001_core.sql"),
    },
    EmbeddedMigration {
        version: 2,
        name: "growth_memory",
        sql: include_str!("../../migrations/0002_growth_memory.sql"),
    },
    EmbeddedMigration {
        version: 3,
        name: "content",
        sql: include_str!("../../migrations/0003_content.sql"),
    },
    EmbeddedMigration {
        version: 4,
        name: "platforms",
        sql: include_str!("../../migrations/0004_platforms.sql"),
    },
    EmbeddedMigration {
        version: 5,
        name: "relationships_jobs",
        sql: include_str!("../../migrations/0005_relationships_jobs.sql"),
    },
    EmbeddedMigration {
        version: 6,
        name: "browser_history",
        sql: include_str!("../../migrations/0006_browser_history.sql"),
    },
];

pub async fn run(pool: &SqlitePool) -> AppResult<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _fgl_migrations (version INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, checksum TEXT NOT NULL, applied_at TEXT NOT NULL)",
    )
    .execute(pool)
    .await?;

    for migration in MIGRATIONS {
        let checksum = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(migration.sql.as_bytes()));
        let applied_checksum = sqlx::query_scalar::<_, String>(
            "SELECT checksum FROM _fgl_migrations WHERE version = ?",
        )
        .bind(migration.version)
        .fetch_optional(pool)
        .await?;
        if let Some(applied_checksum) = applied_checksum {
            if applied_checksum != checksum {
                return Err(AppError::Internal(format!(
                    "embedded migration {} ({}) changed after it was applied",
                    migration.version, migration.name
                )));
            }
            continue;
        }

        let mut transaction = pool.begin().await?;
        sqlx::raw_sql(migration.sql)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "INSERT INTO _fgl_migrations (version, name, checksum, applied_at) VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(migration.version)
        .bind(migration.name)
        .bind(checksum)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
    }
    Ok(())
}
