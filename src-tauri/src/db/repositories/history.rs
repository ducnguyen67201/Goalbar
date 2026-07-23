use chrono::Utc;
use serde_json::{Value, json};
use sqlx::{Row as _, Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use crate::domain::Platform;
use crate::domain::browser::BrowserRunLimits;
use crate::domain::history::{
    ActivityOwnership, HISTORY_SCHEMA_VERSION, HistoryImportResult, HistoryOverview,
    HistoryOverviewPlatform, HistorySourceKind, NormalizedActivityItem, ParsedHistoryArchive,
};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryContextPurpose {
    Voice,
    Icp,
    Content(Platform),
    Learning,
    Reply,
}

#[derive(Debug, Clone)]
pub struct HistoryRepository {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub struct BrowserRunRecord {
    pub source_id: Uuid,
    pub run_id: Uuid,
}

impl HistoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn commit_archive(
        &self,
        archive: &ParsedHistoryArchive,
    ) -> AppResult<HistoryImportResult> {
        let mut transaction = self.pool.begin().await?;
        let metadata = json!({
            "parserVersion": archive.preview.parser_version,
            "schemaVersion": archive.preview.schema_version,
            "categories": archive.preview.categories,
        });
        let (source_id, duplicate_source) = upsert_source(
            &mut transaction,
            SourceInput {
                platform: archive.preview.platform,
                source_kind: HistorySourceKind::Archive,
                ownership: ActivityOwnership::Own,
                display_name: &archive.preview.display_name,
                account_handle: archive.preview.account_handle.as_deref(),
                fingerprint: &archive.preview.source_fingerprint,
                metadata: &metadata,
            },
        )
        .await?;
        let run_id = create_run(
            &mut transaction,
            source_id,
            "running",
            None,
            "Import official account archive",
            &json!({"maximumItems": archive.items.len()}),
        )
        .await?;
        let imported = insert_items(&mut transaction, source_id, &archive.items).await?;
        let skipped = archive.items.len().saturating_sub(imported as usize) as u32;
        finish_run(
            &mut transaction,
            run_id,
            "completed",
            &json!({
                "discovered": archive.items.len(),
                "imported": imported,
                "skipped": skipped,
                "failed": 0,
            }),
            None,
        )
        .await?;
        transaction.commit().await?;
        Ok(HistoryImportResult {
            source_id,
            run_id,
            platform: archive.preview.platform,
            imported,
            skipped,
            warning_count: archive.preview.warnings.len() as u32,
            duplicate_source,
        })
    }

    pub async fn commit_browser_capture(
        &self,
        platform: Platform,
        ownership: ActivityOwnership,
        display_name: &str,
        fingerprint: &str,
        items: &[NormalizedActivityItem],
    ) -> AppResult<HistoryImportResult> {
        let mut transaction = self.pool.begin().await?;
        let metadata = json!({"captureVersion": 1});
        let (source_id, duplicate_source) = upsert_source(
            &mut transaction,
            SourceInput {
                platform,
                source_kind: HistorySourceKind::Browser,
                ownership,
                display_name,
                account_handle: None,
                fingerprint,
                metadata: &metadata,
            },
        )
        .await?;
        let run_id = create_run(
            &mut transaction,
            source_id,
            "running",
            None,
            "Explicit browser capture",
            &json!({"maximumItems": items.len()}),
        )
        .await?;
        let imported = insert_items(&mut transaction, source_id, items).await?;
        let skipped = items.len().saturating_sub(imported as usize) as u32;
        finish_run(
            &mut transaction,
            run_id,
            "completed",
            &json!({"discovered": items.len(), "imported": imported, "skipped": skipped, "failed": 0}),
            None,
        )
        .await?;
        transaction.commit().await?;
        Ok(HistoryImportResult {
            source_id,
            run_id,
            platform,
            imported,
            skipped,
            warning_count: 0,
            duplicate_source,
        })
    }

    pub async fn create_browser_run(
        &self,
        platform: Platform,
        ownership: ActivityOwnership,
        objective: &str,
        limits: &BrowserRunLimits,
        provider: Option<&str>,
    ) -> AppResult<BrowserRunRecord> {
        let now = Utc::now().to_rfc3339();
        let fingerprint = format!(
            "run:{}:{}:{}",
            platform.as_str(),
            ownership.as_str(),
            Uuid::new_v4()
        );
        let mut transaction = self.pool.begin().await?;
        let metadata = json!({"captureVersion": 1, "createdAt": now});
        let (source_id, _) = upsert_source(
            &mut transaction,
            SourceInput {
                platform,
                source_kind: HistorySourceKind::Browser,
                ownership,
                display_name: "Bounded browser collection",
                account_handle: None,
                fingerprint: &fingerprint,
                metadata: &metadata,
            },
        )
        .await?;
        let run_id = create_run(
            &mut transaction,
            source_id,
            "running",
            provider,
            objective,
            &serde_json::to_value(limits)?,
        )
        .await?;
        transaction.commit().await?;
        Ok(BrowserRunRecord { source_id, run_id })
    }

    pub async fn append_browser_batch(
        &self,
        record: &BrowserRunRecord,
        step: u32,
        url: &str,
        items: &[NormalizedActivityItem],
        total_item_count: u32,
    ) -> AppResult<u32> {
        let mut transaction = self.pool.begin().await?;
        let inserted = insert_items(&mut transaction, record.source_id, items).await?;
        let total_item_count = total_item_count.saturating_add(inserted);
        sqlx::query("INSERT INTO browser_checkpoints (id, run_id, step, url, new_item_count, total_item_count, last_item_key, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(run_id, step) DO NOTHING")
            .bind(Uuid::new_v4().to_string())
            .bind(record.run_id.to_string())
            .bind(i64::from(step))
            .bind(url)
            .bind(i64::from(inserted))
            .bind(i64::from(total_item_count))
            .bind(items.last().map(|item| item.dedupe_key.as_str()))
            .bind(Utc::now().to_rfc3339())
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE ingestion_runs SET counts_json = ?, updated_at = ? WHERE id = ?")
            .bind(json!({"imported": total_item_count, "lastBatch": inserted}).to_string())
            .bind(Utc::now().to_rfc3339())
            .bind(record.run_id.to_string())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(inserted)
    }

    pub async fn finish_browser_run(
        &self,
        run_id: Uuid,
        status: &str,
        item_count: u32,
        pause_reason: Option<&str>,
    ) -> AppResult<()> {
        let completed_at =
            matches!(status, "completed" | "failed" | "cancelled").then(|| Utc::now().to_rfc3339());
        sqlx::query("UPDATE ingestion_runs SET status = ?, counts_json = ?, pause_reason = ?, updated_at = ?, completed_at = ? WHERE id = ?")
            .bind(status)
            .bind(json!({"imported": item_count}).to_string())
            .bind(pause_reason)
            .bind(Utc::now().to_rfc3339())
            .bind(completed_at)
            .bind(run_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn overview(&self) -> AppResult<HistoryOverview> {
        let source_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ingestion_sources")
            .fetch_one(&self.pool)
            .await?;
        let item_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activity_items")
            .fetch_one(&self.pool)
            .await?;
        let rows = sqlx::query(
            "SELECT s.platform, COUNT(DISTINCT s.id) AS source_count, COUNT(a.id) AS item_count, COALESCE(SUM(CASE WHEN a.ownership = 'own' THEN 1 ELSE 0 END), 0) AS own_count, COALESCE(SUM(CASE WHEN a.ownership = 'reference' THEN 1 ELSE 0 END), 0) AS reference_count, MAX(COALESCE(a.published_at, a.observed_at)) AS latest_at FROM ingestion_sources s LEFT JOIN activity_items a ON a.source_id = s.id GROUP BY s.platform ORDER BY s.platform",
        )
        .fetch_all(&self.pool)
        .await?;
        let platforms = rows
            .into_iter()
            .map(|row| {
                let platform: String = row.try_get("platform")?;
                Ok(HistoryOverviewPlatform {
                    platform: Platform::parse(&platform)?,
                    source_count: row.try_get::<i64, _>("source_count")? as u32,
                    item_count: row.try_get::<i64, _>("item_count")? as u32,
                    own_item_count: row.try_get::<i64, _>("own_count")? as u32,
                    reference_item_count: row.try_get::<i64, _>("reference_count")? as u32,
                    latest_at: row.try_get("latest_at")?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        Ok(HistoryOverview {
            schema_version: HISTORY_SCHEMA_VERSION,
            source_count: source_count as u32,
            item_count: item_count as u32,
            platforms,
        })
    }

    pub async fn bounded_context(
        &self,
        purpose: HistoryContextPurpose,
        limit: u32,
        max_chars: usize,
    ) -> AppResult<Value> {
        let (predicate, platform) = match purpose {
            HistoryContextPurpose::Voice => (
                "a.ownership = 'own' AND a.item_kind IN ('post', 'comment', 'reply')",
                None,
            ),
            HistoryContextPurpose::Icp => (
                "a.ownership = 'reference' AND a.item_kind IN ('comment', 'reply', 'connection')",
                None,
            ),
            HistoryContextPurpose::Content(platform) => (
                "a.ownership = 'own' AND a.item_kind IN ('post', 'comment', 'reply')",
                Some(platform),
            ),
            HistoryContextPurpose::Learning => (
                "a.item_kind IN ('post', 'comment', 'reply', 'reaction', 'connection')",
                None,
            ),
            HistoryContextPurpose::Reply => (
                "a.ownership = 'reference' AND a.item_kind IN ('comment', 'reply')",
                None,
            ),
        };
        let query = format!(
            "SELECT a.platform, a.item_kind, a.ownership, a.body, a.canonical_url, a.published_at, s.source_kind, s.display_name FROM activity_items a JOIN ingestion_sources s ON s.id = a.source_id WHERE {predicate} AND (? IS NULL OR a.platform = ?) ORDER BY COALESCE(a.published_at, a.observed_at) DESC LIMIT ?"
        );
        let platform_value = platform.map(|value| value.as_str().to_owned());
        let rows = sqlx::query(&query)
            .bind(platform_value.as_deref())
            .bind(platform_value.as_deref())
            .bind(i64::from(limit.min(100)))
            .fetch_all(&self.pool)
            .await?;
        let mut remaining = max_chars;
        let mut excerpts = Vec::new();
        for row in rows {
            let body: String = row.try_get("body")?;
            if body.is_empty() || remaining == 0 {
                continue;
            }
            let excerpt = body.chars().take(remaining.min(2_000)).collect::<String>();
            remaining = remaining.saturating_sub(excerpt.chars().count());
            excerpts.push(json!({
                "platform": row.try_get::<String, _>("platform")?,
                "itemKind": row.try_get::<String, _>("item_kind")?,
                "ownership": row.try_get::<String, _>("ownership")?,
                "excerpt": excerpt,
                "canonicalUrl": row.try_get::<Option<String>, _>("canonical_url")?,
                "publishedAt": row.try_get::<Option<String>, _>("published_at")?,
                "sourceKind": row.try_get::<String, _>("source_kind")?,
                "source": row.try_get::<String, _>("display_name")?,
            }));
        }
        Ok(json!({
            "schemaVersion": HISTORY_SCHEMA_VERSION,
            "purpose": match purpose {
                HistoryContextPurpose::Voice => "voice",
                HistoryContextPurpose::Icp => "icp",
                HistoryContextPurpose::Content(_) => "content",
                HistoryContextPurpose::Learning => "learning",
                HistoryContextPurpose::Reply => "reply",
            },
            "excerpts": excerpts,
            "contextBudgetRemaining": remaining,
            "privateMessagesIncluded": false,
        }))
    }
}

struct SourceInput<'a> {
    platform: Platform,
    source_kind: HistorySourceKind,
    ownership: ActivityOwnership,
    display_name: &'a str,
    account_handle: Option<&'a str>,
    fingerprint: &'a str,
    metadata: &'a Value,
}

async fn upsert_source(
    transaction: &mut Transaction<'_, Sqlite>,
    input: SourceInput<'_>,
) -> AppResult<(Uuid, bool)> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM ingestion_sources WHERE platform = ? AND source_kind = ? AND source_fingerprint = ?",
    )
    .bind(input.platform.as_str())
    .bind(input.source_kind.as_str())
    .bind(input.fingerprint)
    .fetch_optional(&mut **transaction)
    .await?;
    if let Some(existing) = existing {
        return Ok((
            Uuid::parse_str(&existing).map_err(|error| AppError::Internal(error.to_string()))?,
            true,
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ingestion_sources (id, platform, source_kind, ownership, display_name, account_handle, source_fingerprint, metadata_json, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(id.to_string())
        .bind(input.platform.as_str())
        .bind(input.source_kind.as_str())
        .bind(input.ownership.as_str())
        .bind(input.display_name)
        .bind(input.account_handle)
        .bind(input.fingerprint)
        .bind(input.metadata.to_string())
        .bind(Utc::now().to_rfc3339())
        .execute(&mut **transaction)
        .await?;
    Ok((id, false))
}

async fn create_run(
    transaction: &mut Transaction<'_, Sqlite>,
    source_id: Uuid,
    status: &str,
    provider: Option<&str>,
    objective: &str,
    limits: &Value,
) -> AppResult<Uuid> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO ingestion_runs (id, source_id, status, provider, objective, limits_json, counts_json, started_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, '{}', ?, ?)")
        .bind(id.to_string())
        .bind(source_id.to_string())
        .bind(status)
        .bind(provider)
        .bind(objective)
        .bind(limits.to_string())
        .bind(&now)
        .bind(&now)
        .execute(&mut **transaction)
        .await?;
    Ok(id)
}

async fn insert_items(
    transaction: &mut Transaction<'_, Sqlite>,
    source_id: Uuid,
    items: &[NormalizedActivityItem],
) -> AppResult<u32> {
    let mut inserted = 0_u32;
    for item in items {
        let affected = sqlx::query("INSERT INTO activity_items (id, source_id, platform, item_kind, ownership, direction, remote_id, canonical_url, author_handle, counterparty_handle, body, published_at, observed_at, dedupe_key, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(platform, dedupe_key) DO NOTHING")
            .bind(Uuid::new_v4().to_string())
            .bind(source_id.to_string())
            .bind(item.platform.as_str())
            .bind(item.item_kind.as_str())
            .bind(item.ownership.as_str())
            .bind(item.direction.map(|value| value.as_str()))
            .bind(&item.remote_id)
            .bind(&item.canonical_url)
            .bind(&item.author_handle)
            .bind(&item.counterparty_handle)
            .bind(&item.body)
            .bind(&item.published_at)
            .bind(&item.observed_at)
            .bind(&item.dedupe_key)
            .bind(item.metadata.to_string())
            .execute(&mut **transaction)
            .await?
            .rows_affected();
        inserted += affected as u32;
    }
    Ok(inserted)
}

async fn finish_run(
    transaction: &mut Transaction<'_, Sqlite>,
    run_id: Uuid,
    status: &str,
    counts: &Value,
    error_code: Option<&str>,
) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE ingestion_runs SET status = ?, counts_json = ?, error_code = ?, updated_at = ?, completed_at = ? WHERE id = ?")
        .bind(status)
        .bind(counts.to_string())
        .bind(error_code)
        .bind(&now)
        .bind(&now)
        .bind(run_id.to_string())
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{HistoryContextPurpose, HistoryRepository};
    use crate::db::Database;
    use crate::domain::Platform;
    use crate::domain::history::{ActivityItemKind, ActivityOwnership, NormalizedActivityItem};

    #[tokio::test]
    async fn browser_capture_is_idempotent_and_context_is_bounded() {
        let database = Database::in_memory().await.expect("database");
        let repository = HistoryRepository::new(database.pool().clone());
        let item = NormalizedActivityItem {
            platform: Platform::X,
            item_kind: ActivityItemKind::Post,
            ownership: ActivityOwnership::Own,
            direction: None,
            remote_id: Some("1".to_owned()),
            canonical_url: Some("https://x.com/founder/status/1".to_owned()),
            author_handle: Some("founder".to_owned()),
            counterparty_handle: None,
            body: "A durable founder lesson".to_owned(),
            published_at: None,
            observed_at: Utc::now().to_rfc3339(),
            dedupe_key: "same".to_owned(),
            metadata: serde_json::json!({}),
        };
        let first = repository
            .commit_browser_capture(
                Platform::X,
                ActivityOwnership::Own,
                "capture",
                "fingerprint",
                std::slice::from_ref(&item),
            )
            .await
            .expect("first");
        let second = repository
            .commit_browser_capture(
                Platform::X,
                ActivityOwnership::Own,
                "capture",
                "fingerprint",
                &[item],
            )
            .await
            .expect("second");
        assert_eq!(first.imported, 1);
        assert_eq!(second.imported, 0);
        assert!(second.duplicate_source);
        let context = repository
            .bounded_context(HistoryContextPurpose::Voice, 10, 12)
            .await
            .expect("context");
        assert!(context.to_string().len() < 1_000);
    }

    #[tokio::test]
    async fn private_messages_are_excluded_from_unrelated_contexts() {
        let database = Database::in_memory().await.expect("database");
        let repository = HistoryRepository::new(database.pool().clone());
        let item = NormalizedActivityItem {
            platform: Platform::Linkedin,
            item_kind: ActivityItemKind::Message,
            ownership: ActivityOwnership::Reference,
            direction: None,
            remote_id: Some("private-1".to_owned()),
            canonical_url: None,
            author_handle: None,
            counterparty_handle: Some("counterparty".to_owned()),
            body: "synthetic private message sentinel".to_owned(),
            published_at: None,
            observed_at: Utc::now().to_rfc3339(),
            dedupe_key: "private-message".to_owned(),
            metadata: serde_json::json!({"readOnly": true}),
        };
        repository
            .commit_browser_capture(
                Platform::Linkedin,
                ActivityOwnership::Reference,
                "capture",
                "private-fingerprint",
                &[item],
            )
            .await
            .expect("capture");
        for purpose in [
            HistoryContextPurpose::Voice,
            HistoryContextPurpose::Icp,
            HistoryContextPurpose::Content(Platform::Linkedin),
            HistoryContextPurpose::Learning,
            HistoryContextPurpose::Reply,
        ] {
            let context = repository
                .bounded_context(purpose, 10, 10_000)
                .await
                .expect("context");
            assert!(!context.to_string().contains("private message sentinel"));
            assert_eq!(context["privateMessagesIncluded"], false);
        }
    }
}
