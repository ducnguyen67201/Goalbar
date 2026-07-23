use chrono::Utc;
use sqlx::Row as _;
use uuid::Uuid;

use crate::adapters::platform::{
    DirectMessageRequest, PlatformRegistry, PlatformRequestContext, RemoteMessage, ReplyRequest,
};
use crate::db::repositories::platform::PlatformRepository;
use crate::domain::Platform;
use crate::domain::approval::Approval;
use crate::error::{AppError, AppResult};
use crate::secrets::SecretStore;
use crate::services::publishing::PublishingService;
use crate::validation::payload_hash;

#[derive(Debug, Clone)]
pub struct CommunicationService {
    pool: sqlx::SqlitePool,
    platforms: PlatformRegistry,
}

impl CommunicationService {
    pub fn new(pool: sqlx::SqlitePool, platforms: PlatformRegistry) -> Self {
        Self { pool, platforms }
    }

    pub async fn approve(
        &self,
        conversation_id: Uuid,
        body: &str,
        kind: &str,
    ) -> AppResult<Approval> {
        if !matches!(kind, "reply" | "direct_message") {
            return Err(AppError::Validation(
                "outbound approval kind is invalid".to_owned(),
            ));
        }
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations WHERE id = ?")
            .bind(conversation_id.to_string())
            .fetch_one(&self.pool)
            .await?;
        if exists != 1 {
            return Err(AppError::NotFound(format!(
                "conversation {conversation_id}"
            )));
        }
        let approval = Approval::new(kind, conversation_id, body);
        sqlx::query("INSERT INTO approvals (id, subject_type, subject_id, payload_hash, idempotency_key, approved_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(approval.id.to_string())
            .bind(&approval.subject_type)
            .bind(approval.subject_id.to_string())
            .bind(&approval.payload_hash)
            .bind(approval.idempotency_key.to_string())
            .bind(approval.approved_at.to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(approval)
    }

    pub async fn send(
        &self,
        secrets: &dyn SecretStore,
        conversation_id: Uuid,
        approval_id: Uuid,
        body: String,
        recipient_id: Option<String>,
    ) -> AppResult<RemoteMessage> {
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query("SELECT c.account_id, c.platform, c.remote_id, c.kind, c.source, EXISTS(SELECT 1 FROM browser_inbox_ingestions bi WHERE bi.conversation_id = c.id) AS browser_scan, a.payload_hash, a.idempotency_key, a.consumed_at, a.invalidated_at FROM conversations c JOIN approvals a ON a.subject_id = c.id WHERE c.id = ? AND a.id = ?")
            .bind(conversation_id.to_string())
            .bind(approval_id.to_string())
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or_else(|| AppError::NotFound("conversation approval".to_owned()))?;
        if row.try_get::<String, _>("source")? == "email_notification" {
            return Err(AppError::Unsupported(
                "email notifications are signals only; copy the approved text and send it on the platform"
                    .to_owned(),
            ));
        }
        if row.try_get::<i64, _>("browser_scan")? != 0 {
            return Err(AppError::Unsupported(
                "browser inbox scans are previews only; copy the approved text and send it on the platform"
                    .to_owned(),
            ));
        }
        if row.try_get::<String, _>("payload_hash")? != payload_hash(&body)
            || row.try_get::<Option<String>, _>("consumed_at")?.is_some()
            || row
                .try_get::<Option<String>, _>("invalidated_at")?
                .is_some()
        {
            return Err(AppError::Validation(
                "approval does not match this exact outbound text".to_owned(),
            ));
        }
        let idempotency_key: String = row.try_get("idempotency_key")?;
        let account_id = Uuid::parse_str(row.try_get("account_id")?)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let platform = Platform::parse(row.try_get("platform")?)?;
        let remote_id: String = row.try_get("remote_id")?;
        let kind: String = row.try_get("kind")?;
        sqlx::query("UPDATE approvals SET consumed_at = ? WHERE id = ? AND consumed_at IS NULL")
            .bind(Utc::now().to_rfc3339())
            .bind(approval_id.to_string())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;

        let account = PlatformRepository::new(self.pool.clone())
            .get(account_id)
            .await?;
        let secret = secrets
            .load(&account.secret_ref)?
            .ok_or_else(|| AppError::Authentication("platform token is missing".to_owned()))?;
        let context = PlatformRequestContext {
            access_token: PublishingService::access_token(&secret)?,
            account_id: account.remote_account_id,
            display_name: account.display_name,
            scopes: account.scopes,
        };
        let adapter = self.platforms.get(platform);
        let sent = if kind == "direct_message" {
            adapter
                .send_direct_message(
                    &context,
                    DirectMessageRequest {
                        recipient_id: recipient_id.ok_or_else(|| {
                            AppError::Validation("recipient ID is required".to_owned())
                        })?,
                        body: body.clone(),
                        idempotency_key,
                    },
                )
                .await?
        } else {
            adapter
                .reply(
                    &context,
                    ReplyRequest {
                        remote_parent_id: remote_id,
                        body: body.clone(),
                        idempotency_key,
                    },
                )
                .await?
        };
        sqlx::query("INSERT INTO messages (id, conversation_id, remote_id, body, direction, sent_at) VALUES (?, ?, ?, ?, 'outbound', ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(conversation_id.to_string())
            .bind(&sent.remote_id)
            .bind(body)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(sent)
    }
}

#[cfg(test)]
mod tests {
    use crate::adapters::email::RawEmailNotification;
    use crate::app_state::AppState;
    use crate::db::repositories::relationship::RelationshipRepository;
    use crate::error::AppError;
    use crate::services::email_notifications::EmailNotificationService;

    use super::CommunicationService;

    #[tokio::test]
    async fn email_notification_approvals_cannot_be_sent_through_an_adapter() {
        let state = AppState::for_tests().await.expect("state");
        EmailNotificationService::new(state.database.pool().clone())
            .ingest(
                "test",
                vec![RawEmailNotification {
                    source_message_id: "x-reply-1".to_owned(),
                    sender: "X <notify@x.com>".to_owned(),
                    subject: "Ari replied to your post".to_owned(),
                    received_at: "2026-07-23T18:00:00Z".to_owned(),
                    content: "A useful reply\nhttps://x.com/ari/status/1".to_owned(),
                }],
            )
            .await
            .expect("notification ingestion");
        let conversation = RelationshipRepository::new(state.database.pool().clone())
            .conversations()
            .await
            .expect("conversations")
            .remove(0);
        let service =
            CommunicationService::new(state.database.pool().clone(), state.platforms.clone());
        let body = "Thanks for the thoughtful reply.";
        let approval = service
            .approve(conversation.id, body, "reply")
            .await
            .expect("approval");

        let error = service
            .send(
                state.secrets.as_ref(),
                conversation.id,
                approval.id,
                body.to_owned(),
                None,
            )
            .await
            .expect_err("email notification send must be blocked");
        assert!(matches!(error, AppError::Unsupported(_)));

        let consumed_at: Option<String> =
            sqlx::query_scalar("SELECT consumed_at FROM approvals WHERE id = ?")
                .bind(approval.id.to_string())
                .fetch_one(state.database.pool())
                .await
                .expect("approval state");
        assert!(consumed_at.is_none());
    }
}
