use serde_json::Value;

use crate::error::{AppError, AppResult};

pub fn parse_json_lines(input: &str) -> AppResult<Vec<Value>> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(AppError::from))
        .collect()
}

pub fn find_final_text(events: &[Value]) -> AppResult<String> {
    events
        .iter()
        .rev()
        .find_map(find_text)
        .ok_or_else(|| AppError::Agent("agent stream did not contain a final response".to_owned()))
}

fn find_text(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in [
                "structured_output",
                "output_text",
                "final_response",
                "result",
                "text",
            ] {
                if let Some(value) = map.get(key) {
                    match value {
                        Value::String(text) if !text.is_empty() => return Some(text.clone()),
                        Value::Object(_) | Value::Array(_) => return Some(value.to_string()),
                        _ => {}
                    }
                }
            }
            map.values().find_map(find_text)
        }
        Value::Array(values) => values.iter().rev().find_map(find_text),
        _ => None,
    }
}

pub fn parse_structured_text(text: &str) -> AppResult<Value> {
    serde_json::from_str(text).map_err(|error| {
        AppError::Agent(format!("agent returned invalid structured JSON: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::{find_final_text, parse_json_lines, parse_structured_text};

    #[test]
    fn extracts_nested_final_message() {
        let events = parse_json_lines(
            "{\"type\":\"turn.started\"}\n{\"type\":\"item.completed\",\"item\":{\"text\":\"{\\\"ok\\\":true}\"}}\n",
        )
        .expect("events");
        let text = find_final_text(&events).expect("text");
        assert_eq!(parse_structured_text(&text).expect("json")["ok"], true);
    }

    #[test]
    fn rejects_partial_json_line() {
        assert!(parse_json_lines("{\"type\":").is_err());
    }
}
