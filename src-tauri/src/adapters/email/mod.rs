pub mod apple_mail;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use url::Url;

use crate::domain::Platform;
use crate::validation::payload_hash;

const MAX_SUBJECT_CHARS: usize = 500;
const MAX_CONTENT_CHARS: usize = 12_000;
const MAX_EXCERPT_CHARS: usize = 600;
const MAX_DISPLAY_NAME_CHARS: usize = 120;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawEmailNotification {
    pub source_message_id: String,
    pub sender: String,
    pub subject: String,
    pub received_at: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    CommentThread,
    DirectMessage,
}

impl NotificationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CommentThread => "comment_thread",
            Self::DirectMessage => "direct_message",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedEmailNotification {
    pub source_message_id: String,
    pub platform: Platform,
    pub kind: NotificationKind,
    pub display_name: String,
    pub excerpt: String,
    pub remote_url: String,
    pub received_at: DateTime<Utc>,
    pub content_state: &'static str,
}

pub fn parse_notification(raw: &RawEmailNotification) -> Option<ParsedEmailNotification> {
    let platform = platform_from_sender(&raw.sender)?;
    let subject = bounded(&raw.subject, MAX_SUBJECT_CHARS);
    let content = bounded(&raw.content, MAX_CONTENT_CHARS);
    let kind = classify(&subject)?;
    let received_at = DateTime::parse_from_rfc3339(raw.received_at.trim())
        .ok()?
        .with_timezone(&Utc);
    let source_message_id = normalized_message_id(raw, &subject);
    let display_name = actor_from_subject(&subject, platform);
    let (excerpt, content_state) = excerpt(&subject, &content);
    let remote_url = platform_url(&content, platform, kind)
        .unwrap_or_else(|| default_platform_url(platform, kind).to_owned());
    Some(ParsedEmailNotification {
        source_message_id,
        platform,
        kind,
        display_name,
        excerpt,
        remote_url,
        received_at,
        content_state,
    })
}

fn platform_from_sender(sender: &str) -> Option<Platform> {
    let address = sender
        .rsplit_once('<')
        .map_or(sender, |(_, address)| address)
        .trim()
        .trim_end_matches('>')
        .to_ascii_lowercase();
    let domain = address.rsplit_once('@')?.1;
    if domain == "x.com" || domain == "twitter.com" || domain.ends_with(".x.com") {
        Some(Platform::X)
    } else if matches!(domain, "reddit.com" | "redditmail.com")
        || domain.ends_with(".reddit.com")
        || domain.ends_with(".redditmail.com")
    {
        Some(Platform::Reddit)
    } else if domain == "linkedin.com" || domain.ends_with(".linkedin.com") {
        Some(Platform::Linkedin)
    } else {
        None
    }
}

fn classify(subject: &str) -> Option<NotificationKind> {
    let searchable = subject.to_ascii_lowercase();
    let direct_message_terms = [
        "direct message",
        "sent you a message",
        "new message",
        "chat message",
        "chat request",
        "messaged you",
        "inmail",
    ];
    if direct_message_terms
        .iter()
        .any(|term| searchable.contains(term))
    {
        return Some(NotificationKind::DirectMessage);
    }
    let conversation_terms = [
        "replied",
        "reply to",
        "responded",
        "commented",
        "new comment",
        "mentioned you",
        "mention of you",
    ];
    conversation_terms
        .iter()
        .any(|term| searchable.contains(term))
        .then_some(NotificationKind::CommentThread)
}

fn normalized_message_id(raw: &RawEmailNotification, subject: &str) -> String {
    let candidate = raw
        .source_message_id
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>');
    if candidate.is_empty() {
        payload_hash(&format!(
            "{}\n{subject}\n{}",
            raw.sender.trim(),
            raw.received_at.trim()
        ))
    } else {
        bounded(candidate, 998)
    }
}

fn actor_from_subject(subject: &str, platform: Platform) -> String {
    let lower = subject.to_ascii_lowercase();
    for phrase in [
        " sent you",
        " messaged you",
        " commented",
        " replied",
        " responded",
        " mentioned",
    ] {
        if let Some(index) = lower.find(phrase) {
            let candidate = clean_actor(&subject[..index]);
            if !candidate.is_empty() {
                return bounded(&candidate, MAX_DISPLAY_NAME_CHARS);
            }
        }
    }
    if let Some(index) = lower.find(" from ") {
        let candidate = clean_actor(&subject[index + " from ".len()..]);
        if !candidate.is_empty() {
            return bounded(&candidate, MAX_DISPLAY_NAME_CHARS);
        }
    }
    format!(
        "{} notification",
        match platform {
            Platform::X => "X",
            Platform::Reddit => "Reddit",
            Platform::Linkedin => "LinkedIn",
        }
    )
}

fn clean_actor(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character: char| matches!(character, ':' | '-' | '–' | '—'))
        .trim()
        .trim_start_matches("New ")
        .trim_start_matches("new ")
        .to_owned()
}

fn excerpt(subject: &str, content: &str) -> (String, &'static str) {
    let useful = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            !lower.contains("unsubscribe")
                && !lower.contains("manage email")
                && !lower.contains("notification settings")
                && !lower.starts_with("http://")
                && !lower.starts_with("https://")
        })
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    if useful.is_empty() {
        (bounded(subject, MAX_EXCERPT_CHARS), "link_only")
    } else {
        (bounded(&useful, MAX_EXCERPT_CHARS), "notification_excerpt")
    }
}

fn platform_url(content: &str, platform: Platform, _kind: NotificationKind) -> Option<String> {
    content
        .split_whitespace()
        .filter_map(|token| {
            let start = token.find("https://")?;
            let value = token[start..]
                .trim_matches(|character: char| {
                    matches!(
                        character,
                        '<' | '>' | '(' | ')' | '[' | ']' | '"' | '\'' | ',' | ';'
                    )
                })
                .replace("&amp;", "&");
            Url::parse(&value).ok()
        })
        .find_map(|mut url| {
            let host = url.host_str()?.trim_end_matches('.').to_ascii_lowercase();
            let expected = match platform {
                Platform::X => {
                    host == "x.com"
                        || host == "www.x.com"
                        || host == "twitter.com"
                        || host == "www.twitter.com"
                }
                Platform::Reddit => host == "reddit.com" || host == "www.reddit.com",
                Platform::Linkedin => host == "linkedin.com" || host == "www.linkedin.com",
            };
            if !expected || url.scheme() != "https" {
                return None;
            }
            url.set_query(None);
            url.set_fragment(None);
            Some(url.to_string())
        })
}

const fn default_platform_url(platform: Platform, kind: NotificationKind) -> &'static str {
    match (platform, kind) {
        (Platform::X, NotificationKind::CommentThread) => "https://x.com/notifications",
        (Platform::X, NotificationKind::DirectMessage) => "https://x.com/messages",
        (Platform::Reddit, NotificationKind::CommentThread) => {
            "https://www.reddit.com/notifications"
        }
        (Platform::Reddit, NotificationKind::DirectMessage) => {
            "https://www.reddit.com/message/inbox"
        }
        (Platform::Linkedin, NotificationKind::CommentThread) => {
            "https://www.linkedin.com/notifications/"
        }
        (Platform::Linkedin, NotificationKind::DirectMessage) => {
            "https://www.linkedin.com/messaging/"
        }
    }
}

fn bounded(value: &str, maximum: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(maximum)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{NotificationKind, RawEmailNotification, parse_notification};
    use crate::domain::Platform;

    fn email(sender: &str, subject: &str, content: &str) -> RawEmailNotification {
        RawEmailNotification {
            source_message_id: "<message-1@example.test>".to_owned(),
            sender: sender.to_owned(),
            subject: subject.to_owned(),
            received_at: "2026-07-23T18:00:00Z".to_owned(),
            content: content.to_owned(),
        }
    }

    #[test]
    fn parses_x_reply_and_keeps_only_an_x_link() {
        let parsed = parse_notification(&email(
            "X <notify@x.com>",
            "Ari replied to your post",
            "A thoughtful response\nhttps://evil.test/phish\nhttps://x.com/ari/status/1?utm_source=email",
        ))
        .expect("notification");
        assert_eq!(parsed.platform, Platform::X);
        assert_eq!(parsed.kind, NotificationKind::CommentThread);
        assert_eq!(parsed.display_name, "Ari");
        assert_eq!(parsed.remote_url, "https://x.com/ari/status/1");
    }

    #[test]
    fn parses_reddit_chat_and_linkedin_message() {
        let reddit = parse_notification(&email(
            "Reddit <noreply@redditmail.com>",
            "New chat request from u/founder",
            "u/founder wants to chat.",
        ))
        .expect("reddit notification");
        assert_eq!(reddit.platform, Platform::Reddit);
        assert_eq!(reddit.kind, NotificationKind::DirectMessage);

        let linkedin = parse_notification(&email(
            "LinkedIn <messages-noreply@linkedin.com>",
            "Mina sent you a message",
            "Can we compare notes?",
        ))
        .expect("linkedin notification");
        assert_eq!(linkedin.platform, Platform::Linkedin);
        assert_eq!(linkedin.display_name, "Mina");
    }

    #[test]
    fn falls_back_when_a_link_cannot_pass_the_platform_opener() {
        let parsed = parse_notification(&email(
            "Reddit <noreply@redditmail.com>",
            "u/founder replied to your comment",
            "Open the thread\nhttps://old.reddit.com/r/rust/comments/1",
        ))
        .expect("reddit notification");
        assert_eq!(parsed.remote_url, "https://www.reddit.com/notifications");
    }

    #[test]
    fn rejects_spoofed_domains_and_non_conversation_marketing() {
        assert!(
            parse_notification(&email(
                "Fake <notify@x.com.evil.test>",
                "Ari replied to your post",
                "Click now",
            ))
            .is_none()
        );
        assert!(
            parse_notification(&email(
                "LinkedIn <news@linkedin.com>",
                "Top jobs for you",
                "Here are this week's recommendations",
            ))
            .is_none()
        );
    }
}
