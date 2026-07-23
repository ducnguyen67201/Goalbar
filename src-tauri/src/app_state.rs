use std::path::Path;
use std::sync::Arc;

use crate::adapters::agent::AgentRegistry;
use crate::adapters::platform::PlatformRegistry;
use crate::adapters::platform::oauth::OAuthManager;
use crate::browser::BrowserManager;
use crate::conductor::runner::Conductor;
use crate::db::Database;
use crate::error::AppResult;
use crate::secrets::{OsSecretStore, SecretStore};
use crate::services::history::HistorySelectionManager;
use crate::terminal::TerminalManager;

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: Database,
    pub agents: AgentRegistry,
    pub conductor: Conductor,
    pub platforms: PlatformRegistry,
    pub oauth: OAuthManager,
    pub browser: BrowserManager,
    pub history_selections: HistorySelectionManager,
    pub terminals: TerminalManager,
    pub secrets: Arc<dyn SecretStore>,
}

impl AppState {
    pub async fn open(path: &Path) -> AppResult<Self> {
        let database = Database::open(path).await?;
        let agents = AgentRegistry::default();
        Ok(Self {
            database,
            conductor: Conductor::new(agents.clone()),
            agents,
            platforms: PlatformRegistry::default(),
            oauth: OAuthManager::default(),
            browser: BrowserManager::default(),
            history_selections: HistorySelectionManager::default(),
            terminals: TerminalManager::default(),
            secrets: Arc::new(OsSecretStore),
        })
    }

    #[cfg(test)]
    pub async fn for_tests() -> AppResult<Self> {
        let database = Database::in_memory().await?;
        let agents = AgentRegistry::default();
        Ok(Self {
            database,
            conductor: Conductor::new(agents.clone()),
            agents,
            platforms: PlatformRegistry::default(),
            oauth: OAuthManager::default(),
            browser: BrowserManager::default(),
            history_selections: HistorySelectionManager::default(),
            terminals: TerminalManager::default(),
            secrets: Arc::new(crate::secrets::MemorySecretStore::default()),
        })
    }
}
