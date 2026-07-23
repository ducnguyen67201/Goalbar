use schemars::JsonSchema;
use serde::Serialize;

use crate::adapters::agent::AgentStatus;
use crate::db::repositories::founder::FounderRepository;
use crate::db::repositories::platform::ConnectedAccount;
use crate::domain::founder::FounderProfile;
use crate::domain::metrics::GrowthScore;
use crate::error::AppResult;
use crate::services::scoring::ScoringService;
use crate::services::today::{NextAction, TodayService};
use crate::{app_state::AppState, config::SCHEMA_VERSION};

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapState {
    pub schema_version: u32,
    pub founder: Option<FounderProfile>,
    pub agents: Vec<AgentStatus>,
    pub accounts: Vec<ConnectedAccount>,
    pub score: GrowthScore,
    pub next_actions: Vec<NextAction>,
}

pub async fn load(state: &AppState) -> AppResult<BootstrapState> {
    let founder_repository = FounderRepository::new(state.database.pool().clone());
    let platform_repository =
        crate::db::repositories::platform::PlatformRepository::new(state.database.pool().clone());
    let scoring_service = ScoringService::new(state.database.pool().clone());
    let today_service = TodayService::new(state.database.pool().clone());
    let (founder, agents, accounts, score, next_actions) = tokio::join!(
        founder_repository.latest(),
        state.agents.statuses(),
        platform_repository.list(),
        scoring_service.current(),
        today_service.actions(),
    );
    Ok(BootstrapState {
        schema_version: SCHEMA_VERSION,
        founder: founder?,
        agents,
        accounts: accounts?,
        score: score?,
        next_actions: next_actions?,
    })
}
