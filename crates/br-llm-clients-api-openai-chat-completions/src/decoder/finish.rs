use br_llm_clients_common_call::{PayloadError, ProviderBlock, ProviderEvent};

use super::ChatCompletionsDecoder;
use crate::ending::{Ended, Finish};
use crate::stream_error::finish_error_info;
use crate::tool_fragments::HeldCall;

const EMPTY_ARGUMENTS: &str = "{}";

impl ChatCompletionsDecoder {
    pub(super) fn read_finish(
        &mut self,
        finish: Option<Finish>,
        events: &mut Vec<ProviderEvent>,
    ) -> Result<(), PayloadError> {
        let Some(finish) = finish else {
            return Ok(());
        };
        if self.finished {
            return Err(PayloadError::new("the stream holds a second finish reason"));
        }
        self.finished = true;
        let ended = match finish {
            Finish::Error => {
                events.push(ProviderEvent::Error(finish_error_info()));
                return Ok(());
            }
            Finish::Ended(ended) => ended,
        };
        self.close_open_block(events);
        self.emit_held_calls(&ended, events)?;
        events.push(ProviderEvent::Stop(ended.ending(self.refusal_seen)));
        Ok(())
    }

    fn emit_held_calls(
        &mut self,
        ended: &Ended,
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
            if let Some(json) = arguments_text(arguments, ended) {
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

fn arguments_text(arguments: String, ended: &Ended) -> Option<String> {
    if !arguments.trim().is_empty() {
        return Some(arguments);
    }
    if ended.is_length() {
        return None;
    }
    Some(EMPTY_ARGUMENTS.to_owned())
}
