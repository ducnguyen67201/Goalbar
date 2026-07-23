use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct ContextAssembler {
    max_chars: usize,
}

impl ContextAssembler {
    pub fn new(max_chars: usize) -> Self {
        Self { max_chars }
    }

    pub fn assemble(&self, sections: impl IntoIterator<Item = (String, Value)>) -> Value {
        let mut remaining = self.max_chars;
        let mut output = Map::new();
        for (name, value) in sections {
            let serialized = value.to_string();
            if serialized.len() <= remaining {
                remaining -= serialized.len();
                output.insert(name, value);
            }
        }
        output.insert(
            "contextBudgetRemaining".to_owned(),
            Value::from(remaining as u64),
        );
        Value::Object(output)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ContextAssembler;

    #[test]
    fn context_is_stable_and_bounded() {
        let context = ContextAssembler::new(20).assemble([
            ("founder".to_owned(), json!("short")),
            (
                "privateMessages".to_owned(),
                json!("this section is too long"),
            ),
        ]);
        assert!(context.get("founder").is_some());
        assert!(context.get("privateMessages").is_none());
    }
}
