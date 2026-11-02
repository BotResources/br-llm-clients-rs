use std::io::Write;

use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::{ContentBlock, DeltaOp, ModelResponse, ResponseDelta};

pub struct QuietSink;

impl StreamSink for QuietSink {
    fn send<'a>(&'a mut self, _delta: ResponseDelta) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }
}

#[derive(Default)]
pub struct TextDeltaSink {
    pub text_deltas: usize,
}

impl StreamSink for TextDeltaSink {
    fn send<'a>(&'a mut self, delta: ResponseDelta) -> BoxFuture<'a, ()> {
        if let DeltaOp::TextAppended { text, .. } = delta.op() {
            self.text_deltas = self.text_deltas.saturating_add(1);
            print!("{text}");
            let _ = std::io::stdout().flush();
        }
        Box::pin(async {})
    }
}

pub fn answer_text(response: &ModelResponse) -> String {
    response
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text().as_str()),
            _ => None,
        })
        .collect::<String>()
}
