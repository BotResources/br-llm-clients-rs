use br_llm_clients_core::{CallOutcome, ClientError};
use br_llm_messages::{AssistantBlock, Step};

pub fn outcome(label: &str, outcome: &CallOutcome) {
    let report = outcome.report();
    println!("[{label}] model: {:?}", report.model().map(|m| m.as_str()));
    println!("[{label}] request id: {:?}", report.request_id());
    println!(
        "[{label}] time to first event: {:?}, total: {:?}",
        report.time_to_first_event(),
        report.total_duration()
    );
    println!("[{label}] usage: {:?}", report.usage());
    match outcome.result() {
        Ok(step) => step_content(label, step),
        Err(error) => failure(label, error),
    }
}

fn failure(label: &str, error: &ClientError) {
    println!("[{label}] failure: {error}");
    println!("[{label}] failure detail: {error:?}");
}

fn step_content(label: &str, step: &Step) {
    println!("[{label}] finish: {}", step.stop_reason());
    for block in step.content() {
        match block {
            AssistantBlock::Text { text } => println!("[{label}] text: {}", text.as_str()),
            AssistantBlock::Thinking(thinking) => {
                println!(
                    "[{label}] thinking: {} chars",
                    thinking.text.chars().count()
                );
            }
            AssistantBlock::Structured { value } => println!("[{label}] structured: {value}"),
            AssistantBlock::ToolCall(call) => println!(
                "[{label}] tool call: {} {} {}",
                call.id, call.name, call.arguments
            ),
            other => println!("[{label}] other block: {other:?}"),
        }
    }
}
