#![allow(clippy::unwrap_used)]

use goalbar_lib::db::Database;

#[tokio::test]
async fn migration_schema_enforces_platform_enum() {
    let database = Database::in_memory().await.expect("database");
    let result = sqlx::query("INSERT INTO connected_accounts (id, platform, client_id, remote_account_id, display_name, secret_ref, scopes_json, capabilities_json, status, created_at, updated_at) VALUES ('1', 'threads', 'client', 'remote', 'Founder', 'secret', '[]', '{}', 'connected', 'now', 'now')")
        .execute(database.pool())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn migration_six_adds_history_tables_without_changing_operational_data() {
    let database = Database::in_memory().await.expect("database");
    sqlx::query("INSERT INTO app_settings (key, value_json, updated_at) VALUES (?, ?, ?)")
        .bind("migration-test")
        .bind(r#"{"preserved":true}"#)
        .bind("2026-07-22T00:00:00Z")
        .execute(database.pool())
        .await
        .expect("seed operational record");

    let tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('ingestion_sources', 'ingestion_runs', 'activity_items', 'browser_checkpoints')",
    )
    .fetch_one(database.pool())
    .await
    .expect("history tables");
    let setting: String = sqlx::query_scalar("SELECT value_json FROM app_settings WHERE key = ?")
        .bind("migration-test")
        .fetch_one(database.pool())
        .await
        .expect("preserved operational record");
    assert_eq!(tables, 4);
    assert_eq!(setting, r#"{"preserved":true}"#);
}

#[tokio::test]
async fn migration_seven_adds_staged_research_without_changing_operational_data() {
    let database = Database::in_memory().await.expect("database");
    let tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('browser_research_trace', 'browser_research_findings')",
    )
    .fetch_one(database.pool())
    .await
    .expect("research tables");
    assert_eq!(tables, 2);
}
