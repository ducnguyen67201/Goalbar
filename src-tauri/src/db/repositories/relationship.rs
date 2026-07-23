use sqlx::{Row as _, SqlitePool};
use uuid::Uuid;

use crate::domain::relationship::{
    ConversationContentState, ConversationSource, ConversationSummary,
};
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
        let rows = sqlx::query("SELECT c.id, c.platform, c.remote_id, c.kind, COALESCE(r.display_name, c.notification_display_name, 'Unknown person') AS display_name, COALESCE((SELECT body FROM messages m WHERE m.conversation_id = c.id ORDER BY sent_at DESC LIMIT 1), '') AS preview, c.unread_count, c.reply_capability, c.remote_url, CASE WHEN bi.conversation_id IS NOT NULL THEN 'browser_scan' ELSE c.source END AS source, c.content_state, c.updated_at FROM conversations c LEFT JOIN relationships r ON r.id = c.relationship_id LEFT JOIN browser_inbox_ingestions bi ON bi.conversation_id = c.id ORDER BY c.unread_count DESC, c.updated_at DESC")
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
                    kind: row.try_get("kind")?,
                    display_name: row.try_get("display_name")?,
                    preview: row.try_get("preview")?,
                    unread_count: row.try_get::<i64, _>("unread_count")? as u32,
                    reply_capability: state,
                    remote_url: row.try_get("remote_url")?,
                    source: ConversationSource::parse(row.try_get("source")?)?,
                    content_state: ConversationContentState::parse(row.try_get("content_state")?)?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect()
    }

    pub async fn mark_read(&self, conversation_id: Uuid) -> AppResult<bool> {
        let result =
            sqlx::query("UPDATE conversations SET unread_count = 0, seen_at = ? WHERE id = ?")
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(conversation_id.to_string())
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() == 1)
    }
}
