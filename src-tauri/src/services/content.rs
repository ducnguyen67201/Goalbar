use serde_json::json;
use uuid::Uuid;

use crate::adapters::agent::AgentProvider;
use crate::conductor::context::ContextAssembler;
use crate::conductor::prompt::CONTENT_PROMPT;
use crate::conductor::runner::Conductor;
use crate::conductor::task::structured_task;
use crate::db::repositories::content::ContentRepository;
use crate::domain::content::{ContentIdeaInput, GeneratedContentSet};
use crate::domain::founder::FounderProfile;
use crate::error::AppResult;
use crate::services::history::HistoryContextService;

#[derive(Debug, Clone)]
pub struct ContentService {
    conductor: Conductor,
    repository: ContentRepository,
    history: HistoryContextService,
}

impl ContentService {
    pub fn new(conductor: Conductor, pool: sqlx::SqlitePool) -> Self {
        Self {
            conductor,
            repository: ContentRepository::new(pool.clone()),
            history: HistoryContextService::new(pool),
        }
    }

    pub async fn generate(
        &self,
        provider: AgentProvider,
        founder: &FounderProfile,
        input: ContentIdeaInput,
    ) -> AppResult<(Uuid, GeneratedContentSet)> {
        let voice_examples = self.history.voice_examples(10, 5_000).await?;
        let platform_examples = json!({
            "x": self.history.content_examples(crate::domain::Platform::X, 5, 3_000).await?,
            "reddit": self.history.content_examples(crate::domain::Platform::Reddit, 5, 3_000).await?,
            "linkedin": self.history.content_examples(crate::domain::Platform::Linkedin, 5, 3_000).await?,
        });
        let context = ContextAssembler::new(30_000).assemble([
            ("founder".to_owned(), serde_json::to_value(founder)?),
            ("idea".to_owned(), serde_json::to_value(&input)?),
            ("voiceExamples".to_owned(), voice_examples),
            ("platformExamples".to_owned(), platform_examples),
            (
                "platformGuidance".to_owned(),
                json!({
                    "x": "Concise, conversational, strong hook; no invented results.",
                    "reddit": "Context-rich and discussion-first; include a title and avoid promotion.",
                    "linkedin": "Founder narrative with a concrete lesson and whitespace."
                }),
            ),
        ]);
        let job_id = Uuid::new_v4();
        let task =
            structured_task::<GeneratedContentSet>("content_variants", CONTENT_PROMPT, context);
        let (set, agent_result) = self
            .conductor
            .run::<GeneratedContentSet>(job_id, provider, task)
            .await?;
        let idea_id = self
            .repository
            .save_generated_set(
                founder.id,
                input,
                set.clone(),
                agent_result.provider.as_str(),
                &agent_result.provider_version,
            )
            .await?;
        Ok((idea_id, set))
    }
}
