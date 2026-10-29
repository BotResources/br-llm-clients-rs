use br_llm_clients_common_call::{PayloadError, ProviderBlock, ProviderEvent};

use crate::decoder::ChatCompletionsDecoder;
use crate::ending::ending_of;
use crate::stream_error::finish_error_info;
use crate::tool_fragments::HeldCall;

const ERROR_REASON: &str = "error";
const LENGTH_REASON: &str = "length";
const EMPTY_ARGUMENTS: &str = "{}";

impl ChatCompletionsDecoder {
    pub(crate) fn read_finish(
        &mut self,
        finish_reason: Option<&str>,
        events: &mut Vec<ProviderEvent>,
    ) -> Result<(), PayloadError> {
        let Some(reason) = finish_reason else {
            return Ok(());
        };
        if self.finished {
            return Err(PayloadError::new("the stream holds a second finish reason"));
        }
        self.finished = true;
        if reason == ERROR_REASON {
            events.push(ProviderEvent::Error(finish_error_info()));
            return Ok(());
        }
        self.close_open_block(events);
        self.emit_held_calls(reason, events)?;
        events.push(ProviderEvent::Stop(ending_of(reason, self.refusal_seen)?));
        Ok(())
    }

    fn emit_held_calls(
        &mut self,
        reason: &str,
        events: &mut Vec<ProviderEvent>,
    ) -> Result<(), PayloadError> {
        for (index, call) in std::mem::take(&mut self.held) {
            let key = index.to_string();
            let HeldCall {
                id,
                name,
                arguments,
            } = call;
            let Some(name) = name else {
                return Err(PayloadError::new(
                    "a tool call reached the finish with no function name",
                ));
            };
            events.push(ProviderEvent::BlockStart {
                key: key.clone(),
                block: ProviderBlock::ToolCall { name, id },
            });
            if let Some(json) = arguments_text(arguments, reason) {
                events.push(ProviderEvent::ToolArgumentsDelta {
                    key: key.clone(),
                    json,
                });
            }
            events.push(ProviderEvent::BlockEnd { key });
        }
        Ok(())
    }
}

fn arguments_text(arguments: String, reason: &str) -> Option<String> {
    if !arguments.trim().is_empty() {
        return Some(arguments);
    }
    if reason == LENGTH_REASON {
        return None;
    }
    Some(EMPTY_ARGUMENTS.to_owned())
}
