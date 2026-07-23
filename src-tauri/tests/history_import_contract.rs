#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use tagline_lib::adapters::history::{HistoryParserRegistry, file_fingerprint};
use tagline_lib::db::Database;
use tagline_lib::db::repositories::history::HistoryRepository;
use tagline_lib::domain::Platform;
use uuid::Uuid;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/history")
        .join(name)
}

#[tokio::test]
async fn synthetic_platform_archives_preview_and_normalize() {
    let cases = [
        ("x-tweets.txt", Platform::X),
        ("linkedin-shares.csv", Platform::Linkedin),
        ("reddit-comments.csv", Platform::Reddit),
    ];
    for (name, expected_platform) in cases {
        let path = fixture(name);
        let parsed = HistoryParserRegistry::default()
            .parse(
                &path,
                Uuid::new_v4(),
                name,
                &file_fingerprint(&path).expect("fingerprint"),
            )
            .expect("synthetic archive");
        assert_eq!(parsed.preview.platform, expected_platform);
        assert_eq!(parsed.preview.estimated_records, 1);
        assert_eq!(parsed.items.len(), 1);
        assert!(!parsed.items[0].body.is_empty());
    }
}

#[tokio::test]
async fn importing_the_same_archive_is_idempotent() {
    let path = fixture("x-tweets.txt");
    let parsed = HistoryParserRegistry::default()
        .parse(
            &path,
            Uuid::new_v4(),
            "x-tweets.txt",
            &file_fingerprint(&path).expect("fingerprint"),
        )
        .expect("archive");
    let database = Database::in_memory().await.expect("database");
    let repository = HistoryRepository::new(database.pool().clone());

    let first = repository
        .commit_archive(&parsed)
        .await
        .expect("first import");
    let second = repository
        .commit_archive(&parsed)
        .await
        .expect("duplicate import");
    assert_eq!(first.imported, 1);
    assert_eq!(second.imported, 0);
    assert!(second.duplicate_source);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activity_items")
        .fetch_one(database.pool())
        .await
        .expect("count");
    assert_eq!(count, 1);

    sqlx::query("DELETE FROM ingestion_sources WHERE id = ?")
        .bind(first.source_id.to_string())
        .execute(database.pool())
        .await
        .expect("delete source");
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activity_items")
        .fetch_one(database.pool())
        .await
        .expect("remaining item count");
    assert_eq!(remaining, 0);
}
