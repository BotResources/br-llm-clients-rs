use br_llm_clients_core::{CallOutcome, ClientError};
use br_llm_messages::{ContentBlock, ModelResponse};

pub fn outcome(label: &str, outcome: &CallOutcome) {
    let report = outcome.report();
    println!(
        "[{label}] model: {:?}",
        report.model().map(|model| model.as_str())
    );
    println!("[{label}] request id: {:?}", report.request_id());
    println!(
        "[{label}] time to first event: {:?}, total: {:?}",
        report.time_to_first_event(),
        report.total_duration()
    );
    match outcome.result() {
        Ok(response) => blocks(label, response),
        Err(error) => failure(label, error),
    }
}

fn failure(label: &str, error: &ClientError) {
    println!("[{label}] failure: {error}");
    println!("[{label}] failure case: {error:?}");
}

fn blocks(label: &str, response: &ModelResponse) {
    println!("[{label}] ending: {:?}", response.ending());
    for block in response.blocks() {
        match block {
            ContentBlock::Text(text) => println!("[{label}] text: {}", text.text().as_str()),
            ContentBlock::Thinking(thinking) => println!(
                "[{label}] thinking: {} chars",
                thinking.text().chars().count()
            ),
            ContentBlock::StructuredContent(structured) => {
                match structured.value().to_json_string() {
                    Ok(json) => println!("[{label}] structured: {json}"),
                    Err(error) => println!("[{label}] structured: unreadable ({error})"),
                }
            }
            ContentBlock::RuntimeToolCall(call) => println!(
                "[{label}] tool call: {} {:?}",
                call.name().as_str(),
                call.external_id().map(|id| id.as_str())
            ),
            other => println!("[{label}] other block: {:?}", other.kind()),
        }
    }
}

pub fn has_text(response: &ModelResponse) -> bool {
    response
        .blocks()
        .iter()
        .any(|block| matches!(block, ContentBlock::Text(text) if !text.text().as_str().trim().is_empty()))
}
