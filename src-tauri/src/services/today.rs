use schemars::JsonSchema;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NextAction {
    pub kind: String,
    pub title: String,
    pub reason: String,
    pub route: String,
    pub priority: u8,
}

#[derive(Debug, Clone)]
pub struct TodayService {
    pool: SqlitePool,
}

impl TodayService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn actions(&self) -> AppResult<Vec<NextAction>> {
        let founder_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM founder_profiles WHERE onboarding_completed = 1",
        )
        .fetch_one(&self.pool)
        .await?;
        let account_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM connected_accounts WHERE status = 'connected'",
        )
        .fetch_one(&self.pool)
        .await?;
        let approvals: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM content_variants WHERE status = 'approved'")
                .fetch_one(&self.pool)
                .await?;
        let unread: i64 =
            sqlx::query_scalar("SELECT COALESCE(SUM(unread_count), 0) FROM conversations")
                .fetch_one(&self.pool)
                .await?;
        let mut actions = Vec::new();
        if founder_count == 0 {
            actions.push(NextAction {
                kind: "onboarding".to_owned(),
                title: "Teach the lab who you are".to_owned(),
                reason: "Your ICP and voice need a founder baseline.".to_owned(),
                route: "/onboarding".to_owned(),
                priority: 100,
            });
        } else if account_count == 0 {
            actions.push(NextAction {
                kind: "connect".to_owned(),
                title: "Connect your first founder channel".to_owned(),
                reason: "Connections stay local and unlock observation.".to_owned(),
                route: "/settings".to_owned(),
                priority: 90,
            });
        } else {
            actions.push(NextAction {
                kind: "experiment".to_owned(),
                title: "Start one focused content experiment".to_owned(),
                reason: "One measured hypothesis is more useful than more untracked posts."
                    .to_owned(),
                route: "/create".to_owned(),
                priority: 70,
            });
        }
        if unread > 0 {
            actions.push(NextAction {
                kind: "reply".to_owned(),
                title: format!("Review {unread} unread conversations"),
                reason: "Active conversations are stronger growth signals than raw impressions."
                    .to_owned(),
                route: "/inbox".to_owned(),
                priority: 95,
            });
        }
        if approvals > 0 {
            actions.push(NextAction {
                kind: "approval".to_owned(),
                title: format!("Publish {approvals} approved drafts"),
                reason: "These exact revisions are ready for an account decision.".to_owned(),
                route: "/create".to_owned(),
                priority: 85,
            });
        }
        actions.sort_by_key(|action| std::cmp::Reverse(action.priority));
        Ok(actions)
    }
}
