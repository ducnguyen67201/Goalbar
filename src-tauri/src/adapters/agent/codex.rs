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

    fn binary_candidates() -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(path) = std::env::var_os("GOALBAR_CODEX_PATH")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
        {
            push_unique(&mut candidates, path);
        }

        if let Ok(paths) = which::which_all("codex") {
            for path in paths {
                push_unique(&mut candidates, path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            add_nvm_candidates(&mut candidates);
            push_unique(
                &mut candidates,
                PathBuf::from("/Applications/ChatGPT.app/Contents/Resources/codex"),
            );
        }

        candidates
    }

    async fn resolve_binary(&self) -> AppResult<(PathBuf, String)> {
        self.resolve_binary_from(Self::binary_candidates()).await
    }

    async fn resolve_binary_from(&self, candidates: Vec<PathBuf>) -> AppResult<(PathBuf, String)> {
        if candidates.is_empty() {
            return Err(AppError::Agent("Codex CLI is not installed".to_owned()));
        }

        let mut failures = Vec::new();
        for path in candidates {
            let version = probe(&self.runner, &path, vec!["--version".to_owned()]).await;
            if version.0 == 0 {
                return Ok((path, version.1));
            }
            failures.push(format!(
                "{}: {}",
                path.display(),
                concise_failure(&version.1)
            ));
        }

        Err(AppError::Agent(format!(
            "No working Codex CLI was found. {} Reinstall Codex or set GOALBAR_CODEX_PATH to a healthy executable.",
            failures.join(" | ")
        )))
    }
}

#[async_trait]
impl AgentAdapter for CodexAdapter {
    fn provider(&self) -> AgentProvider {
        AgentProvider::Codex
    }

    async fn status(&self) -> AgentStatus {
        let candidates = Self::binary_candidates();
        if candidates.is_empty() {
            return missing_status(AgentProvider::Codex);
        }
        let (path, version) = match self.resolve_binary().await {
            Ok(resolved) => resolved,
            Err(error) => {
                return AgentStatus {
                    provider: AgentProvider::Codex,
                    readiness: AgentReadiness::Incompatible,
                    path: candidates.into_iter().next(),
                    version: None,
                    detail: Some(error.to_string()),
                };
            }
        };
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
            version: (!version.is_empty()).then_some(version),
            detail: (auth.0 != 0).then_some("Run `codex login` to authenticate.".to_owned()),
        }
    }

    async fn run(
        &self,
        task: &StructuredAgentTask,
        cancellation: CancellationToken,
    ) -> AppResult<AgentResult> {
        let (path, version) = self.resolve_binary().await?;
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
            provider_version: version,
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

fn push_unique(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() && !candidates.contains(&path) {
        candidates.push(path);
    }
}

#[cfg(target_os = "macos")]
fn add_nvm_candidates(candidates: &mut Vec<PathBuf>) {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let versions_dir = PathBuf::from(home).join(".nvm/versions/node");
    let Ok(entries) = std::fs::read_dir(versions_dir) else {
        return;
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("bin/codex"))
        .collect::<Vec<_>>();
    paths.sort();
    paths.reverse();
    for path in paths {
        push_unique(candidates, path);
    }
}

fn concise_failure(message: &str) -> String {
    const MAX_CHARS: usize = 180;
    let collapsed = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= MAX_CHARS {
        return collapsed;
    }
    format!("{}…", collapsed.chars().take(MAX_CHARS).collect::<String>())
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CodexAdapter;
    use crate::adapters::agent::process::ProcessRunner;

    #[cfg(unix)]
    #[tokio::test]
    async fn skips_a_broken_codex_candidate() {
        let adapter = CodexAdapter::new(ProcessRunner);
        let (selected, version) = adapter
            .resolve_binary_from(vec![
                PathBuf::from("/bin/false"),
                PathBuf::from("/bin/echo"),
            ])
            .await
            .expect("healthy fallback");

        assert_eq!(selected, PathBuf::from("/bin/echo"));
        assert_eq!(version, "--version");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reports_when_every_codex_candidate_is_broken() {
        let adapter = CodexAdapter::new(ProcessRunner);
        let error = adapter
            .resolve_binary_from(vec![PathBuf::from("/bin/false")])
            .await
            .expect_err("broken candidate");

        assert!(error.to_string().contains("No working Codex CLI"));
        assert!(error.to_string().contains("GOALBAR_CODEX_PATH"));
    }

    #[test]
    fn truncates_verbose_spawn_failures() {
        let message = "spawn failure ".repeat(40);
        let concise = super::concise_failure(&message);
        assert!(concise.chars().count() <= 181);
        assert!(concise.ends_with('…'));
    }
}
