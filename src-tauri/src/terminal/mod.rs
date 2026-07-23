use std::collections::HashMap;
use std::fmt;
use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::Utc;
use portable_pty::{
    ChildKiller, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem as _,
};
use tauri::{AppHandle, Emitter as _};
use uuid::Uuid;

use crate::domain::terminal::{
    TerminalExitEvent, TerminalKind, TerminalOutputEvent, TerminalSession, TerminalStatus,
};
use crate::error::{AppError, AppResult};

const MAX_TERMINAL_SESSIONS: usize = 4;
const MAX_TERMINAL_INPUT_BYTES: usize = 64 * 1024;
const MAX_TERMINAL_DIMENSION: u16 = 500;
const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;

struct ManagedTerminal {
    summary: TerminalSession,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn std::io::Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

impl fmt::Debug for ManagedTerminal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedTerminal")
            .field("summary", &self.summary)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Default)]
pub struct TerminalManager {
    inner: Arc<Mutex<HashMap<Uuid, ManagedTerminal>>>,
}

impl fmt::Debug for TerminalManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TerminalManager")
            .field("session_count", &self.list().len())
            .finish()
    }
}

impl TerminalManager {
    pub fn list(&self) -> Vec<TerminalSession> {
        let mut sessions = self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(|session| session.summary.clone())
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        sessions
    }

    pub fn create(
        &self,
        app: &AppHandle,
        kind: TerminalKind,
        rows: u16,
        cols: u16,
    ) -> AppResult<TerminalSession> {
        validate_size(rows, cols)?;
        if self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
            >= MAX_TERMINAL_SESSIONS
        {
            return Err(AppError::Validation(format!(
                "Goalbar supports at most {MAX_TERMINAL_SESSIONS} terminal panes"
            )));
        }

        let executable = executable_for(kind)?;
        let working_directory = std::env::current_dir().map_err(AppError::Io)?;
        let pair = NativePtySystem::default()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| AppError::Internal(format!("could not open terminal: {error}")))?;
        let mut command = CommandBuilder::new(&executable);
        command.cwd(&working_directory);
        command.env("TERM", "xterm-256color");
        let mut child = pair.slave.spawn_command(command).map_err(|error| {
            AppError::Agent(format!("could not start {}: {error}", kind.title()))
        })?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| AppError::Internal(format!("could not read terminal: {error}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| AppError::Internal(format!("could not write terminal: {error}")))?;
        let killer = child.clone_killer();
        let id = Uuid::new_v4();
        let summary = TerminalSession {
            id,
            kind,
            title: kind.title().to_owned(),
            status: TerminalStatus::Running,
            working_directory: display_path(&working_directory),
            created_at: Utc::now().to_rfc3339(),
        };
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                id,
                ManagedTerminal {
                    summary: summary.clone(),
                    master: pair.master,
                    writer,
                    killer,
                },
            );

        let output_app = app.clone();
        std::thread::spawn(move || {
            let mut buffer = [0_u8; OUTPUT_CHUNK_BYTES];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(length) => {
                        let _ = output_app.emit_to(
                            "main",
                            "terminal://output",
                            TerminalOutputEvent {
                                session_id: id,
                                data: String::from_utf8_lossy(&buffer[..length]).into_owned(),
                            },
                        );
                    }
                    Err(_) => break,
                }
            }
        });

        let wait_app = app.clone();
        let sessions = self.inner.clone();
        std::thread::spawn(move || {
            let status = child.wait();
            let (terminal_status, exit_code) = match status {
                Ok(status) => (TerminalStatus::Exited, Some(status.exit_code())),
                Err(_) => (TerminalStatus::Failed, None),
            };
            if let Some(session) = sessions
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get_mut(&id)
            {
                session.summary.status = terminal_status;
            }
            let _ = wait_app.emit_to(
                "main",
                "terminal://exit",
                TerminalExitEvent {
                    session_id: id,
                    status: terminal_status,
                    exit_code,
                },
            );
        });

        Ok(summary)
    }

    pub fn write(&self, id: Uuid, data: &str) -> AppResult<()> {
        if data.len() > MAX_TERMINAL_INPUT_BYTES {
            return Err(AppError::Validation(
                "terminal input exceeds the 64 KiB command limit".to_owned(),
            ));
        }
        let mut sessions = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let session = sessions
            .get_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("terminal session {id}")))?;
        if session.summary.status != TerminalStatus::Running {
            return Err(AppError::Validation(
                "the terminal process is no longer running".to_owned(),
            ));
        }
        session.writer.write_all(data.as_bytes())?;
        session.writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, id: Uuid, rows: u16, cols: u16) -> AppResult<()> {
        validate_size(rows, cols)?;
        let sessions = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let session = sessions
            .get(&id)
            .ok_or_else(|| AppError::NotFound(format!("terminal session {id}")))?;
        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| AppError::Internal(format!("could not resize terminal: {error}")))
    }

    pub fn close(&self, id: Uuid) -> AppResult<()> {
        let mut session = self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id)
            .ok_or_else(|| AppError::NotFound(format!("terminal session {id}")))?;
        if session.summary.status == TerminalStatus::Running {
            session.killer.kill()?;
        }
        Ok(())
    }
}

fn validate_size(rows: u16, cols: u16) -> AppResult<()> {
    if !(2..=MAX_TERMINAL_DIMENSION).contains(&rows)
        || !(2..=MAX_TERMINAL_DIMENSION).contains(&cols)
    {
        return Err(AppError::Validation(
            "terminal rows and columns must be between 2 and 500".to_owned(),
        ));
    }
    Ok(())
}

fn executable_for(kind: TerminalKind) -> AppResult<PathBuf> {
    let executable = match kind {
        TerminalKind::Bash => shell_executable(),
        TerminalKind::Codex => PathBuf::from("codex"),
        TerminalKind::Claude => PathBuf::from("claude"),
    };
    if executable.is_absolute() && executable.is_file() {
        return Ok(executable);
    }
    which::which(&executable).map_err(|_| {
        AppError::Agent(format!(
            "{} is not installed or is not available on PATH",
            kind.title()
        ))
    })
}

#[cfg(unix)]
fn shell_executable() -> PathBuf {
    std::env::var_os("SHELL")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"))
}

#[cfg(windows)]
fn shell_executable() -> PathBuf {
    PathBuf::from("powershell.exe")
}

fn display_path(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::validate_size;

    #[test]
    fn terminal_dimensions_are_bounded() {
        assert!(validate_size(24, 80).is_ok());
        assert!(validate_size(1, 80).is_err());
        assert!(validate_size(24, 501).is_err());
    }
}
