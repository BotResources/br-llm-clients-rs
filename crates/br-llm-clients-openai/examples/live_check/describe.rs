use br_llm_clients_core::{BoxFuture, CallOutcome, StreamSink};
use br_llm_messages::{Step, StreamEvent};

#[derive(Default)]
pub struct CountingSink {
    pub events: usize,
}

impl StreamSink for CountingSink {
    fn send<'a>(&'a mut self, _event: StreamEvent) -> BoxFuture<'a, ()> {
        self.events += 1;
        Box::pin(async {})
    }
}

pub fn describe(outcome: &CallOutcome, events: usize) -> Option<&Step> {
    let report = outcome.report();
    println!("  stream events: {events}");
    println!("  answering model: {:?}", report.model());
    println!("  request id: {:?}", report.request_id());
    println!("  time to first event: {:?}", report.time_to_first_event());
    println!("  total duration: {:?}", report.total_duration());
    println!("  usage: {:?}", report.usage());
    match outcome.result() {
        Ok(step) => {
            println!("  finish: {}", step.stop_reason());
            println!("  thinking blocks: {}", step.thinking().count());
            for text in step.text() {
                println!("  text: {text}");
            }
            for call in step.tool_calls() {
                println!("  tool call: {} {} {}", call.id, call.name, call.arguments);
            }
            for value in step.structured() {
                println!("  structured: {value}");
            }
            Some(step)
        }
        Err(error) => {
            println!("  error: {error:?}");
            println!("  retryable: {}", error.is_retryable());
            None
        }
    }
}
