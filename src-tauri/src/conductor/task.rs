use schemars::{JsonSchema, schema_for};
use serde_json::Value;

use crate::adapters::agent::StructuredAgentTask;

pub fn structured_task<T: JsonSchema>(
    kind: &str,
    prompt: &str,
    context: Value,
) -> StructuredAgentTask {
    StructuredAgentTask {
        task_kind: kind.to_owned(),
        prompt: prompt.to_owned(),
        context,
        output_schema: serde_json::to_value(schema_for!(T))
            .unwrap_or_else(|_| serde_json::json!({"type": "object"})),
        timeout_seconds: 120,
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::content::GeneratedContentSet;

    use super::structured_task;

    #[test]
    fn task_contains_generated_schema() {
        let task =
            structured_task::<GeneratedContentSet>("content", "draft", serde_json::json!({}));
        assert!(task.output_schema.get("properties").is_some());
    }
}
