use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TerminalKind {
    Bash,
    Codex,
    Claude,
}

impl TerminalKind {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Bash => "Shell",
            Self::Codex => "Codex",
            Self::Claude => "Claude",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TerminalStatus {
    Running,
    Exited,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSession {
    pub id: Uuid,
    pub kind: TerminalKind,
    pub title: String,
    pub status: TerminalStatus,
    pub working_directory: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputEvent {
    pub session_id: Uuid,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalExitEvent {
    pub session_id: Uuid,
    pub status: TerminalStatus,
    pub exit_code: Option<u32>,
}
