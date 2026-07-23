#![allow(clippy::unwrap_used)]

use tagline_lib::db::Database;

#[tokio::test]
async fn migration_schema_enforces_platform_enum() {
    let database = Database::in_memory().await.expect("database");
    let result = sqlx::query("INSERT INTO connected_accounts (id, platform, client_id, remote_account_id, display_name, secret_ref, scopes_json, capabilities_json, status, created_at, updated_at) VALUES ('1', 'threads', 'client', 'remote', 'Founder', 'secret', '[]', '{}', 'connected', 'now', 'now')")
        .execute(database.pool())
        .await;
    assert!(result.is_err());
}
