use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::jsonl::parse_structured_text;
use super::process::{ProcessRequest, ProcessRunner};
use super::{
    AgentAdapter, AgentProvider, AgentReadiness, AgentResult, AgentStatus, StructuredAgentTask,
};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct ClaudeAdapter {
    runner: ProcessRunner,
}

impl ClaudeAdapter {
    pub fn new(runner: ProcessRunner) -> Self {
        Self { runner }
    }

    fn binary_path() -> Option<PathBuf> {
        which::which("claude").ok()
    }
}

#[async_trait]
impl AgentAdapter for ClaudeAdapter {
    fn provider(&self) -> AgentProvider {
        AgentProvider::Claude
    }

    async fn status(&self) -> AgentStatus {
        let Some(path) = Self::binary_path() else {
            return AgentStatus {
                provider: AgentProvider::Claude,
                readiness: AgentReadiness::Missing,
                path: None,
                version: None,
                detail: Some("Install Claude Code to use this provider.".to_owned()),
            };
        };
        let version = probe(&self.runner, &path, vec!["--version".to_owned()]).await;
        let auth = probe(
            &self.runner,
            &path,
            vec!["auth".to_owned(), "status".to_owned()],
        )
        .await;
        AgentStatus {
            provider: AgentProvider::Claude,
            readiness: if auth.0 == 0 {
                AgentReadiness::Ready
            } else {
                AgentReadiness::AuthRequired
            },
            path: Some(path),
            version: (!version.1.is_empty()).then_some(version.1),
            detail: (auth.0 != 0).then_some("Run `claude auth login` to authenticate.".to_owned()),
        }
    }

    async fn run(
        &self,
        task: &StructuredAgentTask,
        cancellation: CancellationToken,
    ) -> AppResult<AgentResult> {
        let path = Self::binary_path()
            .ok_or_else(|| AppError::Agent("Claude Code is not installed".to_owned()))?;
        let prompt = serde_json::json!({
            "task": task.task_kind,
            "instructions": task.prompt,
            "context": task.context,
            "output": "Return only an object matching the supplied JSON Schema."
        })
        .to_string();
        let output = self
            .runner
            .run(
                ProcessRequest {
                    program: path,
                    args: vec![
                        "-p".to_owned(),
                        "--output-format".to_owned(),
                        "json".to_owned(),
                        "--json-schema".to_owned(),
                        task.output_schema.to_string(),
                        "--tools".to_owned(),
                        String::new(),
                        "--permission-mode".to_owned(),
                        "dontAsk".to_owned(),
                    ],
                    stdin: prompt,
                    timeout: Duration::from_secs(task.timeout_seconds.clamp(5, 300)),
                    environment: BTreeMap::new(),
                },
                cancellation,
            )
            .await?;
        if output.status != 0 {
            return Err(AppError::Agent(format!(
                "Claude exited with {}: {}",
                output.status,
                output.stderr.trim()
            )));
        }
        let envelope: Value = serde_json::from_str(&output.stdout)?;
        let structured = envelope
            .get("structured_output")
            .or_else(|| envelope.get("structuredOutput"))
            .cloned()
            .or_else(|| {
                envelope
                    .get("result")
                    .and_then(Value::as_str)
                    .and_then(|value| parse_structured_text(value).ok())
            })
            .unwrap_or(envelope.clone());
        Ok(AgentResult {
            provider: AgentProvider::Claude,
            provider_version: self
                .status()
                .await
                .version
                .unwrap_or_else(|| "unknown".to_owned()),
            output: structured,
            usage: envelope.get("usage").cloned(),
        })
    }
}

async fn probe(runner: &ProcessRunner, path: &std::path::Path, args: Vec<String>) -> (i32, String) {
    match runner
        .run(
            ProcessRequest {
                program: path.to_path_buf(),
                args,
                stdin: String::new(),
                timeout: Duration::from_secs(8),
                environment: BTreeMap::new(),
            },
            CancellationToken::new(),
        )
        .await
    {
        Ok(output) => (
            output.status,
            format!("{} {}", output.stdout.trim(), output.stderr.trim())
                .trim()
                .to_owned(),
        ),
        Err(error) => (-1, error.to_string()),
    }
}
