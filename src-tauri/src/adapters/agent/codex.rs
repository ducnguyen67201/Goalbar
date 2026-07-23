use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::jsonl::{find_final_text, parse_json_lines, parse_structured_text};
use super::process::{ProcessRequest, ProcessRunner};
use super::{
    AgentAdapter, AgentProvider, AgentReadiness, AgentResult, AgentStatus, StructuredAgentTask,
};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct CodexAdapter {
    runner: ProcessRunner,
}

impl CodexAdapter {
    pub fn new(runner: ProcessRunner) -> Self {
        Self { runner }
    }

    fn binary_path() -> Option<PathBuf> {
        which::which("codex").ok()
    }
}

#[async_trait]
impl AgentAdapter for CodexAdapter {
    fn provider(&self) -> AgentProvider {
        AgentProvider::Codex
    }

    async fn status(&self) -> AgentStatus {
        let Some(path) = Self::binary_path() else {
            return missing_status(AgentProvider::Codex);
        };
        let version = probe(&self.runner, &path, vec!["--version".to_owned()]).await;
        let auth = probe(
            &self.runner,
            &path,
            vec!["login".to_owned(), "status".to_owned()],
        )
        .await;
        AgentStatus {
            provider: AgentProvider::Codex,
            readiness: if auth.0 == 0 {
                AgentReadiness::Ready
            } else {
                AgentReadiness::AuthRequired
            },
            path: Some(path),
            version: (!version.1.is_empty()).then_some(version.1),
            detail: (auth.0 != 0).then_some("Run `codex login` to authenticate.".to_owned()),
        }
    }

    async fn run(
        &self,
        task: &StructuredAgentTask,
        cancellation: CancellationToken,
    ) -> AppResult<AgentResult> {
        let path = Self::binary_path()
            .ok_or_else(|| AppError::Agent("Codex CLI is not installed".to_owned()))?;
        let mut schema_file = tempfile::NamedTempFile::new()?;
        schema_file.write_all(task.output_schema.to_string().as_bytes())?;
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
                        "exec".to_owned(),
                        "--json".to_owned(),
                        "--sandbox".to_owned(),
                        "read-only".to_owned(),
                        "--skip-git-repo-check".to_owned(),
                        "--output-schema".to_owned(),
                        schema_file.path().to_string_lossy().into_owned(),
                        "-".to_owned(),
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
                "Codex exited with {}: {}",
                output.status,
                output.stderr.trim()
            )));
        }
        let events = parse_json_lines(&output.stdout)?;
        let text = find_final_text(&events)?;
        let usage = events
            .iter()
            .rev()
            .find_map(|value| value.get("usage").cloned());
        Ok(AgentResult {
            provider: AgentProvider::Codex,
            provider_version: self
                .status()
                .await
                .version
                .unwrap_or_else(|| "unknown".to_owned()),
            output: parse_structured_text(&text)?,
            usage,
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

fn missing_status(provider: AgentProvider) -> AgentStatus {
    AgentStatus {
        provider,
        readiness: AgentReadiness::Missing,
        path: None,
        version: None,
        detail: Some(format!(
            "Install the {} CLI to use this provider.",
            provider.binary()
        )),
    }
}
