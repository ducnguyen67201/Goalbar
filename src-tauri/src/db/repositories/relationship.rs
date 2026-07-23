use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::domain::relationship::ConversationSummary;
use crate::domain::{CapabilityState, Platform};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct RelationshipRepository {
    pool: SqlitePool,
}

impl RelationshipRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn conversations(&self) -> AppResult<Vec<ConversationSummary>> {
        let rows = sqlx::query("SELECT c.id, c.platform, c.remote_id, COALESCE(r.display_name, 'Unknown person') AS display_name, COALESCE((SELECT body FROM messages m WHERE m.conversation_id = c.id ORDER BY sent_at DESC LIMIT 1), '') AS preview, c.unread_count, c.reply_capability, c.remote_url FROM conversations c LEFT JOIN relationships r ON r.id = c.relationship_id ORDER BY c.unread_count DESC, c.updated_at DESC")
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|row| {
                let state = match row.try_get::<&str, _>("reply_capability")? {
                    "supported" => CapabilityState::Supported,
                    "unsupported" => CapabilityState::Unsupported,
                    "approval_pending" => CapabilityState::ApprovalPending,
                    _ => CapabilityState::Unknown,
                };
                Ok(ConversationSummary {
                    id: Uuid::parse_str(row.try_get("id")?)
                        .map_err(|error| AppError::Internal(error.to_string()))?,
                    platform: Platform::parse(row.try_get("platform")?)?,
                    remote_id: row.try_get("remote_id")?,
                    display_name: row.try_get("display_name")?,
                    preview: row.try_get("preview")?,
                    unread_count: row.try_get::<i64, _>("unread_count")? as u32,
                    reply_capability: state,
                    remote_url: row.try_get("remote_url")?,
                })
            })
            .collect()
    }
}
