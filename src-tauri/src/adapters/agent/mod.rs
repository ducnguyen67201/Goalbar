pub mod claude;
pub mod codex;
pub mod jsonl;
pub mod process;

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, AppResult};

use self::claude::ClaudeAdapter;
use self::codex::CodexAdapter;
use self::process::ProcessRunner;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AgentProvider {
    Codex,
    Claude,
}

impl AgentProvider {
    pub const ALL: [Self; 2] = [Self::Codex, Self::Claude];

    pub const fn binary(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }

    pub const fn as_str(self) -> &'static str {
        self.binary()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentReadiness {
    Missing,
    Installed,
    AuthRequired,
    Ready,
    Incompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub provider: AgentProvider,
    pub readiness: AgentReadiness,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredAgentTask {
    pub task_kind: String,
    pub prompt: String,
    pub context: Value,
    pub output_schema: Value,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentResult {
    pub provider: AgentProvider,
    pub provider_version: String,
    pub output: Value,
    pub usage: Option<Value>,
}

#[async_trait]
pub trait AgentAdapter: Debug + Send + Sync {
    fn provider(&self) -> AgentProvider;
    async fn status(&self) -> AgentStatus;
    async fn run(
        &self,
        task: &StructuredAgentTask,
        cancellation: CancellationToken,
    ) -> AppResult<AgentResult>;
}

#[derive(Debug, Clone)]
pub struct AgentRegistry {
    codex: Arc<CodexAdapter>,
    claude: Arc<ClaudeAdapter>,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        let runner = ProcessRunner;
        Self {
            codex: Arc::new(CodexAdapter::new(runner.clone())),
            claude: Arc::new(ClaudeAdapter::new(runner)),
        }
    }
}

impl AgentRegistry {
    pub async fn statuses(&self) -> Vec<AgentStatus> {
        let (codex, claude) = tokio::join!(self.codex.status(), self.claude.status());
        vec![codex, claude]
    }

    pub fn get(&self, provider: AgentProvider) -> Arc<dyn AgentAdapter> {
        match provider {
            AgentProvider::Codex => self.codex.clone(),
            AgentProvider::Claude => self.claude.clone(),
        }
    }

    pub fn parse_provider(value: &str) -> AppResult<AgentProvider> {
        match value {
            "codex" => Ok(AgentProvider::Codex),
            "claude" => Ok(AgentProvider::Claude),
            _ => Err(AppError::Validation(format!(
                "unknown agent provider: {value}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentProvider, AgentRegistry};

    #[test]
    fn parses_only_supported_agents() {
        assert_eq!(
            AgentRegistry::parse_provider("codex").ok(),
            Some(AgentProvider::Codex)
        );
        assert!(AgentRegistry::parse_provider("openrouter").is_err());
    }
}
