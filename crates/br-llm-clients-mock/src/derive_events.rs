use br_llm_messages::{AssistantBlock, BlockKind, Step, StreamEvent};

use crate::build_error::MockBuildError;
use crate::chunking::{Chunking, pieces};

#[derive(Debug, Clone, PartialEq)]
pub struct DerivedEvents {
    events: Vec<StreamEvent>,
}

impl DerivedEvents {
    pub fn events(&self) -> &[StreamEvent] {
        &self.events
    }

    pub fn before_finish(&self) -> usize {
        self.events.len().saturating_sub(1)
    }

    pub(crate) fn into_events(self) -> Vec<StreamEvent> {
        self.events
    }
}

pub fn derive_events(step: &Step, chunking: &Chunking) -> Result<DerivedEvents, MockBuildError> {
    chunking.check()?;
    let mut events = Vec::new();
    for (index, block) in step.content().iter().enumerate() {
        block_events(&mut events, index, block, chunking);
    }
    events.push(StreamEvent::Finish {
        stop_reason: step.stop_reason().clone(),
        usage: step.usage().cloned(),
    });
    Ok(DerivedEvents { events })
}

fn block_events(
    events: &mut Vec<StreamEvent>,
    index: usize,
    block: &AssistantBlock,
    chunking: &Chunking,
) {
    match block {
        AssistantBlock::Text { text } => {
            start(events, index, BlockKind::Text);
            for piece in pieces(text.as_str(), chunking.text_size()) {
                events.push(StreamEvent::TextDelta {
                    index,
                    text: piece.to_owned(),
                });
            }
        }
        AssistantBlock::Thinking(thinking) => {
            start(events, index, BlockKind::Thinking);
            for piece in pieces(&thinking.text, chunking.thinking_size()) {
                events.push(StreamEvent::ThinkingDelta {
                    index,
                    text: piece.to_owned(),
                });
            }
            if let Some(signature) = &thinking.signature {
                events.push(StreamEvent::SignatureDelta {
                    index,
                    signature: signature.as_str().to_owned(),
                });
            }
        }
        AssistantBlock::RedactedThinking(redacted) => {
            start(events, index, BlockKind::RedactedThinking);
            events.push(StreamEvent::RedactedThinkingData {
                index,
                data: redacted.data().to_owned(),
            });
        }
        AssistantBlock::Structured { value } => {
            start(events, index, BlockKind::Structured);
            for piece in pieces(&value.to_string(), chunking.structured_size()) {
                events.push(StreamEvent::StructuredDelta {
                    index,
                    json_fragment: piece.to_owned(),
                });
            }
        }
        AssistantBlock::ToolCall(call) => {
            events.push(StreamEvent::ToolCallStart {
                index,
                id: call.id.clone(),
                name: call.name.clone(),
            });
            for piece in pieces(&call.arguments.to_string(), chunking.tool_arguments_size()) {
                events.push(StreamEvent::ToolCallArgumentsDelta {
                    index,
                    json_fragment: piece.to_owned(),
                });
            }
        }
    }
    events.push(StreamEvent::BlockEnd { index });
}

fn start(events: &mut Vec<StreamEvent>, index: usize, kind: BlockKind) {
    events.push(StreamEvent::BlockStart { index, kind });
}
