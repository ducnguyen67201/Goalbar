use chrono::Utc;
use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::browser::policy::{browser_url, page_kind, platform_from_url, strip_tracking};
use crate::domain::browser::{BrowserPageKind, SavedBrowserReply, SavedBrowserReplyStatus};
use crate::error::{AppError, AppResult};
use crate::validation::payload_hash;

const MAX_REPLY_CHARS: usize = 8_000;

#[derive(Debug, Clone)]
pub struct BrowserReplyRepository {
    pool: SqlitePool,
}

impl BrowserReplyRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save_prepared(
        &self,
        target_url: &str,
        exact_reply: &str,
    ) -> AppResult<SavedBrowserReply> {
        validate_exact_reply(exact_reply)?;
        let target = strip_tracking(browser_url(target_url)?);
        if page_kind(&target) != BrowserPageKind::Post {
            return Err(AppError::Validation(
                "saved browser replies require an exact post URL".to_owned(),
            ));
        }
        let platform = platform_from_url(&target).ok_or_else(|| {
            AppError::Validation("saved browser replies require a supported platform".to_owned())
        })?;
        let saved = SavedBrowserReply {
            id: Uuid::new_v4(),
            platform,
            target_url: target.to_string(),
            exact_reply: exact_reply.to_owned(),
            status: SavedBrowserReplyStatus::Prepared,
            prepared_at: Utc::now().to_rfc3339(),
            confirmed_posted_at: None,
        };
        sqlx::query("INSERT INTO saved_browser_replies (id, platform, target_url, exact_reply, payload_hash, status, prepared_at, confirmed_posted_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(saved.id.to_string())
            .bind(saved.platform.as_str())
            .bind(&saved.target_url)
            .bind(&saved.exact_reply)
            .bind(payload_hash(&saved.exact_reply))
            .bind(saved.status.as_str())
            .bind(&saved.prepared_at)
            .bind(&saved.confirmed_posted_at)
            .execute(&self.pool)
            .await?;
        Ok(saved)
    }

    pub async fn list_recent(&self, maximum: u32) -> AppResult<Vec<SavedBrowserReply>> {
        let limit = i64::from(maximum.clamp(1, 100));
        let rows = sqlx::query("SELECT id, platform, target_url, exact_reply, status, prepared_at, confirmed_posted_at FROM saved_browser_replies ORDER BY prepared_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("id")?;
                let platform: String = row.try_get("platform")?;
                let status: String = row.try_get("status")?;
                Ok(SavedBrowserReply {
                    id: Uuid::parse_str(&id).map_err(|error| {
                        AppError::Internal(format!("invalid saved reply id: {error}"))
                    })?,
                    platform: crate::domain::Platform::parse(&platform)?,
                    target_url: row.try_get("target_url")?,
                    exact_reply: row.try_get("exact_reply")?,
                    status: SavedBrowserReplyStatus::parse(&status)?,
                    prepared_at: row.try_get("prepared_at")?,
                    confirmed_posted_at: row.try_get("confirmed_posted_at")?,
                })
            })
            .collect()
    }
}

fn validate_exact_reply(value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(
            "the exact reply cannot be empty".to_owned(),
        ));
    }
    if value.chars().count() > MAX_REPLY_CHARS {
        return Err(AppError::Validation(format!(
            "the exact reply cannot exceed {MAX_REPLY_CHARS} characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::BrowserReplyRepository;
    use crate::db::Database;
    use crate::domain::browser::SavedBrowserReplyStatus;

    #[tokio::test]
    async fn saves_each_exact_prepared_reply_as_a_distinct_local_record() {
        let database = Database::in_memory().await.expect("database");
        let repository = BrowserReplyRepository::new(database.pool().clone());
        let url = "https://x.com/founder/status/123?tracking=1";
        let reply = "First line.\n\nSecond line.";

        let first = repository
            .save_prepared(url, reply)
            .await
            .expect("first saved reply");
        let second = repository
            .save_prepared(url, reply)
            .await
            .expect("second saved reply");
        let recent = repository.list_recent(10).await.expect("recent replies");

        assert_ne!(first.id, second.id);
        assert_eq!(recent.len(), 2);
        assert!(recent.iter().all(|item| {
            item.exact_reply == reply
                && item.target_url == "https://x.com/founder/status/123"
                && item.status == SavedBrowserReplyStatus::Prepared
                && item.confirmed_posted_at.is_none()
        }));
    }

    #[tokio::test]
    async fn rejects_non_post_targets_and_blank_replies() {
        let database = Database::in_memory().await.expect("database");
        let repository = BrowserReplyRepository::new(database.pool().clone());

        assert!(
            repository
                .save_prepared("https://x.com/home", "A reply")
                .await
                .is_err()
        );
        assert!(
            repository
                .save_prepared("https://x.com/founder/status/123", " \n ")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn preserves_platform_identity_for_x_linkedin_and_reddit() {
        let database = Database::in_memory().await.expect("database");
        let repository = BrowserReplyRepository::new(database.pool().clone());
        let targets = [
            ("https://x.com/founder/status/123", "x"),
            (
                "https://www.linkedin.com/posts/founder_update-123",
                "linkedin",
            ),
            (
                "https://www.reddit.com/r/startups/comments/123/lesson/",
                "reddit",
            ),
        ];

        for (target, expected_platform) in targets {
            let saved = repository
                .save_prepared(target, "An exact prepared reply.")
                .await
                .expect("saved reply");
            assert_eq!(saved.platform.as_str(), expected_platform);
        }
    }
}
