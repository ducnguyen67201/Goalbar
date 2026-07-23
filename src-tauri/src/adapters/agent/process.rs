use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, AppResult};

const MAX_STREAM_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ProcessRequest {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub stdin: String,
    pub timeout: Duration,
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessRunner;

impl ProcessRunner {
    pub async fn run(
        &self,
        request: ProcessRequest,
        cancellation: CancellationToken,
    ) -> AppResult<ProcessOutput> {
        let mut command = Command::new(&request.program);
        command
            .args(&request.args)
            .env_clear()
            .envs(minimum_environment())
            .envs(request.environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn().map_err(|error| {
            AppError::Agent(format!(
                "could not start {}: {error}",
                request.program.display()
            ))
        })?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(request.stdin.as_bytes()).await?;
            stdin.shutdown().await?;
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Agent("agent stdout was unavailable".to_owned()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::Agent("agent stderr was unavailable".to_owned()))?;
        let stdout_task = tokio::spawn(read_limited(stdout));
        let stderr_task = tokio::spawn(read_limited(stderr));

        let status = tokio::select! {
            result = child.wait() => result?,
            () = cancellation.cancelled() => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(AppError::Cancelled);
            }
            () = tokio::time::sleep(request.timeout) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(AppError::Timeout(format!("{} exceeded {} seconds", request.program.display(), request.timeout.as_secs())));
            }
        };
        let stdout = stdout_task
            .await
            .map_err(|error| AppError::Agent(error.to_string()))??;
        let stderr = stderr_task
            .await
            .map_err(|error| AppError::Agent(error.to_string()))??;
        Ok(ProcessOutput {
            status: status.code().unwrap_or(-1),
            stdout,
            stderr,
        })
    }
}

async fn read_limited(mut stream: impl AsyncRead + Unpin) -> AppResult<String> {
    let mut output = Vec::new();
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        if output.len() + read > MAX_STREAM_BYTES {
            return Err(AppError::Agent(
                "agent output exceeded the 2 MiB limit".to_owned(),
            ));
        }
        output.extend_from_slice(&chunk[..read]);
    }
    String::from_utf8(output)
        .map_err(|error| AppError::Agent(format!("agent output was not UTF-8: {error}")))
}

fn minimum_environment() -> BTreeMap<String, String> {
    const ALLOWED: &[&str] = &[
        "PATH",
        "HOME",
        "USERPROFILE",
        "LOCALAPPDATA",
        "APPDATA",
        "TMPDIR",
        "TEMP",
        "CODEX_HOME",
        "CLAUDE_CONFIG_DIR",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
    ];
    ALLOWED
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| ((*key).to_owned(), value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;

    use super::{ProcessRequest, ProcessRunner};

    #[cfg(unix)]
    #[tokio::test]
    async fn captures_stdout_without_a_shell() {
        let output = ProcessRunner
            .run(
                ProcessRequest {
                    program: PathBuf::from("/bin/echo"),
                    args: vec!["hello".to_owned()],
                    stdin: String::new(),
                    timeout: Duration::from_secs(2),
                    environment: BTreeMap::new(),
                },
                CancellationToken::new(),
            )
            .await
            .expect("process output");
        assert_eq!(output.stdout.trim(), "hello");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn honours_cancellation() {
        let token = CancellationToken::new();
        token.cancel();
        let result = ProcessRunner
            .run(
                ProcessRequest {
                    program: PathBuf::from("/bin/sleep"),
                    args: vec!["5".to_owned()],
                    stdin: String::new(),
                    timeout: Duration::from_secs(10),
                    environment: BTreeMap::new(),
                },
                token,
            )
            .await;
        assert!(matches!(result, Err(crate::error::AppError::Cancelled)));
    }
}
