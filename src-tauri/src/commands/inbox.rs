use serde::Deserialize;
use sqlx::Row as _;
use tauri::State;
use uuid::Uuid;

use crate::adapters::agent::AgentRegistry;
use crate::adapters::platform::RemoteMessage;
use crate::app_state::AppState;
use crate::conductor::context::ContextAssembler;
use crate::conductor::prompt::REPLY_PROMPT;
use crate::conductor::task::structured_task;
use crate::db::repositories::relationship::RelationshipRepository;
use crate::domain::approval::Approval;
use crate::domain::relationship::{ConversationSummary, ReplyOptions};
use crate::error::{AppError, CommandError};
use crate::services::communication::CommunicationService;
use crate::services::history::HistoryContextService;

#[tauri::command]
pub async fn list_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<ConversationSummary>, CommandError> {
    RelationshipRepository::new(state.database.pool().clone())
        .conversations()
        .await
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftReplyInput {
    pub provider: String,
    pub conversation_id: String,
}

#[tauri::command]
pub async fn draft_reply(
    input: DraftReplyInput,
    state: State<'_, AppState>,
) -> Result<ReplyOptions, CommandError> {
    let conversation_id = parse_uuid(&input.conversation_id)?;
    let provider = AgentRegistry::parse_provider(&input.provider).map_err(CommandError::from)?;
    let rows = sqlx::query("SELECT direction, body, sent_at FROM messages WHERE conversation_id = ? ORDER BY sent_at DESC LIMIT 20")
        .bind(conversation_id.to_string())
        .fetch_all(state.database.pool())
        .await
        .map_err(AppError::from)
        .map_err(CommandError::from)?;
    let messages = rows
        .iter()
        .rev()
        .map(|row| {
            serde_json::json!({
                "direction": row.try_get::<String, _>("direction").unwrap_or_default(),
                "body": row.try_get::<String, _>("body").unwrap_or_default(),
                "sentAt": row.try_get::<String, _>("sent_at").unwrap_or_default()
            })
        })
        .collect::<Vec<_>>();
    let founder =
        crate::db::repositories::founder::FounderRepository::new(state.database.pool().clone())
            .latest()
            .await
            .map_err(CommandError::from)?;
    let history = HistoryContextService::new(state.database.pool().clone())
        .reply_evidence(12, 4_000)
        .await
        .map_err(CommandError::from)?;
    let context = ContextAssembler::new(20_000).assemble([
        (
            "founder".to_owned(),
            serde_json::to_value(founder)
                .map_err(AppError::from)
                .map_err(CommandError::from)?,
        ),
        (
            "messages".to_owned(),
            serde_json::to_value(messages)
                .map_err(AppError::from)
                .map_err(CommandError::from)?,
        ),
        ("historyEvidence".to_owned(), history),
    ]);
    let task = structured_task::<ReplyOptions>("reply_options", REPLY_PROMPT, context);
    state
        .conductor
        .run::<ReplyOptions>(Uuid::new_v4(), provider, task)
        .await
        .map(|(options, _)| options)
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApproveReplyInput {
    pub conversation_id: String,
    pub body: String,
}

#[tauri::command]
pub async fn approve_reply(
    input: ApproveReplyInput,
    state: State<'_, AppState>,
) -> Result<Approval, CommandError> {
    let conversation_id = parse_uuid(&input.conversation_id)?;
    let kind: String = sqlx::query_scalar("SELECT CASE WHEN kind = 'direct_message' THEN 'direct_message' ELSE 'reply' END FROM conversations WHERE id = ?")
        .bind(conversation_id.to_string())
        .fetch_optional(state.database.pool())
        .await
        .map_err(AppError::from)
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::from(AppError::NotFound(format!("conversation {conversation_id}"))))?;
    CommunicationService::new(state.database.pool().clone(), state.platforms.clone())
        .approve(conversation_id, &input.body, &kind)
        .await
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendReplyInput {
    pub conversation_id: String,
    pub approval_id: String,
    pub body: String,
    pub recipient_id: Option<String>,
}

#[tauri::command]
pub async fn send_reply(
    input: SendReplyInput,
    state: State<'_, AppState>,
) -> Result<RemoteMessage, CommandError> {
    CommunicationService::new(state.database.pool().clone(), state.platforms.clone())
        .send(
            state.secrets.as_ref(),
            parse_uuid(&input.conversation_id)?,
            parse_uuid(&input.approval_id)?,
            input.body,
            input.recipient_id,
        )
        .await
        .map_err(CommandError::from)
}

fn parse_uuid(value: &str) -> Result<Uuid, CommandError> {
    Uuid::parse_str(value)
        .map_err(|error| CommandError::from(AppError::Validation(error.to_string())))
}
