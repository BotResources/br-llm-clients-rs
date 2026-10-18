use std::collections::HashSet;

use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::{ToolCallId, WireMessage};

use crate::failure_map::malformed;

const PREFIX: &str = "brgen_";

pub fn is_generated_id(id: &str) -> bool {
    id.starts_with(PREFIX)
}

pub(crate) struct ToolIds {
    used: HashSet<String>,
    next: u64,
}

impl ToolIds {
    pub(crate) fn of_request(request: &ClientRequest) -> Self {
        let used = request
            .messages()
            .iter()
            .filter_map(|message| match message {
                WireMessage::Assistant { content } => Some(content),
                _ => None,
            })
            .flatten()
            .filter_map(|block| block.as_tool_call())
            .map(|call| call.id.as_str().to_owned())
            .collect();
        Self { used, next: 0 }
    }

    pub(crate) fn claim(&mut self, id: &str) -> Result<ToolCallId, ClientError> {
        if is_generated_id(id) {
            return Err(malformed(
                "a tool call id of the provider uses the reserved prefix",
            ));
        }
        if !self.used.insert(id.to_owned()) {
            return Err(malformed("a tool call id is used more than once"));
        }
        ToolCallId::new(id).map_err(|_| malformed("a tool call id is not valid"))
    }

    pub(crate) fn generate(&mut self) -> Result<ToolCallId, ClientError> {
        loop {
            let candidate = format!("{PREFIX}{}", self.next);
            self.next = self.next.saturating_add(1);
            if self.used.insert(candidate.clone()) {
                return ToolCallId::new(candidate)
                    .map_err(|_| malformed("a generated tool call id is not valid"));
            }
        }
    }
}
