#![allow(clippy::unwrap_used)]

use goalbar_lib::adapters::agent::jsonl::{
    find_final_text, parse_json_lines, parse_structured_text,
};

#[test]
fn codex_jsonl_fixture_produces_structured_domain_output() {
    let events = parse_json_lines(
        r#"{"type":"turn.started"}
{"type":"item.completed","item":{"type":"agent_message","text":"{\"variants\":[]}"}}
{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5}}
"#,
    )
    .expect("JSONL fixture");
    let text = find_final_text(&events).expect("final text");
    let value = parse_structured_text(&text).expect("structured output");
    assert!(value["variants"].is_array());
}
