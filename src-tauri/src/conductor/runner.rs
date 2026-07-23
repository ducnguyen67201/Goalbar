use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde::de::DeserializeOwned;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::adapters::agent::{AgentProvider, AgentRegistry, AgentResult, StructuredAgentTask};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct Conductor {
    agents: AgentRegistry,
    cancellations: Arc<Mutex<HashMap<Uuid, CancellationToken>>>,
}

impl Conductor {
    pub fn new(agents: AgentRegistry) -> Self {
        Self {
            agents,
            cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run<T: DeserializeOwned>(
        &self,
        job_id: Uuid,
        provider: AgentProvider,
        task: StructuredAgentTask,
    ) -> AppResult<(T, AgentResult)> {
        let cancellation = CancellationToken::new();
        self.cancellations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(job_id, cancellation.clone());
        let result = self.agents.get(provider).run(&task, cancellation).await;
        self.cancellations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&job_id);
        let result = result?;
        let parsed = serde_json::from_value(result.output.clone()).map_err(|error| {
            AppError::Agent(format!(
                "structured output failed domain validation: {error}"
            ))
        })?;
        Ok((parsed, result))
    }

    pub fn cancel(&self, job_id: Uuid) -> bool {
        self.cancellations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&job_id)
            .map(|token| {
                token.cancel();
                true
            })
            .unwrap_or(false)
    }
}
