use crate::adapters::email::RawEmailNotification;
use crate::error::{AppError, AppResult};

#[cfg(target_os = "macos")]
const APPLE_MAIL_SCRIPT: &str = r#"
const Mail = Application("Mail");
const messages = Mail.inbox().messages();
const maximumCandidatesPerEdge = 300;
const allowedMarkers = ["@x.com", "@twitter.com", "@reddit.com", "@redditmail.com", "@linkedin.com"];
const indices = [];
const edgeCount = Math.min(messages.length, maximumCandidatesPerEdge);

for (let index = 0; index < edgeCount; index += 1) {
  indices.push(index);
}
for (let index = Math.max(edgeCount, messages.length - edgeCount); index < messages.length; index += 1) {
  indices.push(index);
}

const seen = new Set();
const notifications = [];
for (const index of indices) {
  if (seen.has(index) || notifications.length >= 250) continue;
  seen.add(index);
  const message = messages[index];
  let sender = "";
  try { sender = String(message.sender() || ""); } catch (_) { continue; }
  const lowerSender = sender.toLowerCase();
  if (!allowedMarkers.some((marker) => lowerSender.includes(marker))) continue;

  let sourceMessageId = "";
  let subject = "";
  let receivedAt = "";
  let content = "";
  try { sourceMessageId = String(message.messageId() || ""); } catch (_) {}
  try { subject = String(message.subject() || ""); } catch (_) {}
  try {
    const received = message.dateReceived();
    receivedAt = received ? received.toISOString() : "";
  } catch (_) {}
  try { content = String(message.content() || "").slice(0, 12000); } catch (_) {}
  notifications.push({ sourceMessageId, sender, subject, receivedAt, content });
}

JSON.stringify(notifications);
"#;

#[cfg(target_os = "macos")]
pub async fn read_notifications() -> AppResult<Vec<RawEmailNotification>> {
    use tokio::process::Command;

    let mut command = Command::new("/usr/bin/osascript");
    command
        .args(["-l", "JavaScript", "-e", APPLE_MAIL_SCRIPT])
        .kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(20), command.output())
        .await
        .map_err(|_| AppError::Timeout("reading Apple Mail notifications".to_owned()))??;
    if !output.status.success() {
        return Err(AppError::Permission(
            "Apple Mail could not be read. Open Mail, then allow Goalbar to control it in System Settings → Privacy & Security → Automation."
                .to_owned(),
        ));
    }
    let json = String::from_utf8(output.stdout)
        .map_err(|_| AppError::Internal("Apple Mail returned invalid text".to_owned()))?;
    serde_json::from_str(&json).map_err(AppError::from)
}

#[cfg(not(target_os = "macos"))]
pub async fn read_notifications() -> AppResult<Vec<RawEmailNotification>> {
    Err(AppError::Unsupported(
        "the free local email connector currently supports Apple Mail on macOS".to_owned(),
    ))
}
