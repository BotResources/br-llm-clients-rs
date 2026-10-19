use serde_json::Value;

pub(crate) const REDACTED_THINKING_REAL: &str =
    include_str!("recordings/anthropic-redacted-thinking.sse");

pub(crate) const REDACTED_BLOCK_COUNT: usize = 22;

fn data_lines(stream: &str) -> Vec<Value> {
    stream
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|text| serde_json::from_str(text).expect("a JSON data line"))
        .collect()
}

pub(crate) fn recorded_redacted_data(stream: &str) -> Vec<String> {
    data_lines(stream)
        .iter()
        .filter(|value| value["type"] == "content_block_start")
        .filter(|value| value["content_block"]["type"] == "redacted_thinking")
        .map(|value| {
            value["content_block"]["data"]
                .as_str()
                .expect("a data string")
                .to_owned()
        })
        .collect()
}

pub(crate) fn recorded_text_pieces(stream: &str) -> Vec<String> {
    data_lines(stream)
        .iter()
        .filter(|value| value["delta"]["type"] == "text_delta")
        .map(|value| {
            value["delta"]["text"]
                .as_str()
                .expect("a text string")
                .to_owned()
        })
        .collect()
}
