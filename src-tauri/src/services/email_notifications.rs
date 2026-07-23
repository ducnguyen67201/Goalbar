use chrono::Utc;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::adapters::email::apple_mail;
use crate::adapters::email::{ParsedEmailNotification, RawEmailNotification, parse_notification};
use crate::domain::Platform;
use crate::domain::relationship::{EmailNotificationPlatformCounts, EmailNotificationSyncResult};
use crate::error::AppResult;

const LOCAL_EMAIL_CLIENT_ID: &str = "goalbar-email-notifications";
const LOCAL_EMAIL_REMOTE_ACCOUNT_ID: &str = "__goalbar_email_notifications__";
const LOCAL_CAPABILITIES_JSON: &str = r#"{"authenticate":"unsupported","publish":"unsupported","readOwnContent":"unsupported","metrics":"unsupported","reply":"unsupported","directMessages":"unsupported","detail":"Email notifications are local signals. Open the platform to verify and send."}"#;

#[derive(Debug, Clone)]
pub struct EmailNotificationService {
    pool: sqlx::SqlitePool,
}

impl EmailNotificationService {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn sync_apple_mail(&self) -> AppResult<EmailNotificationSyncResult> {
        let notifications = apple_mail::read_notifications().await?;
        self.ingest("apple_mail", notifications).await
    }

    pub async fn ingest(
        &self,
        source: &str,
        raw_notifications: Vec<RawEmailNotification>,
    ) -> AppResult<EmailNotificationSyncResult> {
        let scanned = raw_notifications.len() as u32;
        let mut ignored = 0_u32;
        let mut parsed = Vec::new();
        for raw in raw_notifications {
            if let Some(notification) = parse_notification(&raw) {
                parsed.push(notification);
            } else {
                ignored += 1;
            }
        }

        let mut imported = 0_u32;
        let mut duplicates = 0_u32;
        let mut platform_counts = EmailNotificationPlatformCounts::empty();
        let mut transaction = self.pool.begin().await?;
        for notification in parsed {
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_notification_ingestions WHERE source_message_id = ?",
            )
            .bind(&notification.source_message_id)
            .fetch_one(&mut *transaction)
            .await?;
            if exists != 0 {
                duplicates += 1;
                continue;
            }

            ensure_local_account(&mut transaction, notification.platform).await?;
            insert_notification(&mut transaction, &notification).await?;
            platform_counts.increment(notification.platform);
            imported += 1;
        }
        transaction.commit().await?;

        Ok(EmailNotificationSyncResult {
            source: source.to_owned(),
            scanned,
            imported,
            ignored,
            duplicates,
            platform_counts,
            last_checked_at: Utc::now().to_rfc3339(),
        })
    }
}

async fn ensure_local_account(
    transaction: &mut Transaction<'_, Sqlite>,
    platform: Platform,
) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO connected_accounts (id, platform, client_id, remote_account_id, display_name, secret_ref, scopes_json, capabilities_json, token_expires_at, status, created_at, updated_at) VALUES (?, ?, ?, ?, 'Email notifications', ?, '[]', ?, NULL, 'connected', ?, ?) ON CONFLICT(platform, remote_account_id) DO NOTHING")
        .bind(notification_account_id(platform))
        .bind(platform.as_str())
        .bind(LOCAL_EMAIL_CLIENT_ID)
        .bind(LOCAL_EMAIL_REMOTE_ACCOUNT_ID)
        .bind(format!("local/email-notifications/{}", platform.as_str()))
        .bind(LOCAL_CAPABILITIES_JSON)
        .bind(&now)
        .bind(&now)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

async fn insert_notification(
    transaction: &mut Transaction<'_, Sqlite>,
    notification: &ParsedEmailNotification,
) -> AppResult<()> {
    let conversation_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO conversations (id, account_id, relationship_id, platform, remote_id, kind, unread_count, reply_capability, remote_url, updated_at, source, content_state, notification_display_name, seen_at) VALUES (?, ?, NULL, ?, ?, ?, 1, 'unsupported', ?, ?, 'email_notification', ?, ?, NULL)")
        .bind(conversation_id.to_string())
        .bind(notification_account_id(notification.platform))
        .bind(notification.platform.as_str())
        .bind(format!("email:{}", notification.source_message_id))
        .bind(notification.kind.as_str())
        .bind(&notification.remote_url)
        .bind(notification.received_at.to_rfc3339())
        .bind(notification.content_state)
        .bind(&notification.display_name)
        .execute(&mut **transaction)
        .await?;
    sqlx::query("INSERT INTO messages (id, conversation_id, remote_id, sender_remote_id, body, direction, sent_at) VALUES (?, ?, ?, NULL, ?, 'inbound', ?)")
        .bind(Uuid::new_v4().to_string())
        .bind(conversation_id.to_string())
        .bind(format!("email:{}", notification.source_message_id))
        .bind(&notification.excerpt)
        .bind(notification.received_at.to_rfc3339())
        .execute(&mut **transaction)
        .await?;
    sqlx::query("INSERT INTO email_notification_ingestions (source_message_id, platform, conversation_id, classification, received_at, ingested_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&notification.source_message_id)
        .bind(notification.platform.as_str())
        .bind(conversation_id.to_string())
        .bind(notification.kind.as_str())
        .bind(notification.received_at.to_rfc3339())
        .bind(now)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

const fn notification_account_id(platform: Platform) -> &'static str {
    match platform {
        Platform::X => "00000000-0000-4000-8000-000000000101",
        Platform::Reddit => "00000000-0000-4000-8000-000000000102",
        Platform::Linkedin => "00000000-0000-4000-8000-000000000103",
    }
}

#[cfg(test)]
mod tests {
    use crate::adapters::email::RawEmailNotification;
    use crate::db::Database;
    use crate::db::repositories::platform::PlatformRepository;
    use crate::db::repositories::relationship::RelationshipRepository;
    use crate::domain::relationship::ConversationSource;
    use crate::services::today::TodayService;

    use super::EmailNotificationService;

    #[tokio::test]
    async fn ingestion_is_idempotent_and_creates_local_inbox_rows() {
        let database = Database::in_memory().await.expect("database");
        let service = EmailNotificationService::new(database.pool().clone());
        let email = RawEmailNotification {
            source_message_id: "reddit-message-1".to_owned(),
            sender: "Reddit <noreply@redditmail.com>".to_owned(),
            subject: "u/founder replied to your comment".to_owned(),
            received_at: "2026-07-23T18:00:00Z".to_owned(),
            content: "This is useful context.\nhttps://www.reddit.com/r/rust/comments/1".to_owned(),
        };
        let first = service
            .ingest("test", vec![email.clone()])
            .await
            .expect("first import");
        let second = service
            .ingest("test", vec![email])
            .await
            .expect("second import");
        assert_eq!(first.imported, 1);
        assert_eq!(second.duplicates, 1);

        let repository = RelationshipRepository::new(database.pool().clone());
        let conversations = repository.conversations().await.expect("conversations");
        assert_eq!(conversations.len(), 1);
        assert_eq!(
            conversations[0].source,
            ConversationSource::EmailNotification
        );
        assert_eq!(conversations[0].kind, "comment_thread");
        assert_eq!(conversations[0].unread_count, 1);

        assert!(
            repository
                .mark_read(conversations[0].id)
                .await
                .expect("mark read")
        );
        assert!(
            !repository
                .mark_read(uuid::Uuid::new_v4())
                .await
                .expect("missing conversation")
        );
        assert_eq!(
            repository.conversations().await.expect("updated inbox")[0].unread_count,
            0
        );

        let visible_accounts = PlatformRepository::new(database.pool().clone())
            .list()
            .await
            .expect("visible accounts");
        assert!(visible_accounts.is_empty());

        sqlx::query("INSERT INTO founder_profiles (id, name, product_name, offer, expertise, goals_json, boundaries_json, onboarding_completed, created_at, updated_at) VALUES ('founder', 'Founder', 'Goalbar', 'Local growth', 'Product', '[]', '[]', 1, 'now', 'now')")
            .execute(database.pool())
            .await
            .expect("founder");
        let actions = TodayService::new(database.pool().clone())
            .actions()
            .await
            .expect("today actions");
        assert!(actions.iter().any(|action| action.kind == "connect"));
        assert!(!actions.iter().any(|action| action.kind == "experiment"));
    }
}
