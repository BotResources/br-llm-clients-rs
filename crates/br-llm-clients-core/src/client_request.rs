use std::collections::HashSet;

use br_llm_messages::{Text, WireMessage};

use crate::build_error::BuildError;
use crate::client_error::ClientError;
use crate::message_rules;
use crate::output_mode::OutputMode;
use crate::tool::ToolSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct ClientRequest {
    system: Option<Text>,
    messages: Vec<WireMessage>,
    tools: Vec<ToolSpec>,
    output: OutputMode,
}

impl ClientRequest {
    pub fn new(
        system: Option<Text>,
        messages: Vec<WireMessage>,
        tools: Vec<ToolSpec>,
        output: OutputMode,
    ) -> Result<Self, BuildError> {
        check_messages_shape(&messages)?;
        check_tool_names(&tools)?;
        Ok(Self {
            system,
            messages,
            tools,
            output,
        })
    }

    pub fn system(&self) -> Option<&Text> {
        self.system.as_ref()
    }

    pub fn messages(&self) -> &[WireMessage] {
        &self.messages
    }

    pub fn tools(&self) -> &[ToolSpec] {
        &self.tools
    }

    pub fn output(&self) -> &OutputMode {
        &self.output
    }

    pub fn check_messages(&self) -> Result<(), ClientError> {
        message_rules::check(&self.messages)
    }
}

fn check_messages_shape(messages: &[WireMessage]) -> Result<(), BuildError> {
    let Some((last, before)) = messages.split_last() else {
        return Err(BuildError::NoMessage);
    };
    let relay_before_end = before
        .iter()
        .any(|message| matches!(message, WireMessage::Relay { .. }));
    let lone_relay = before.is_empty() && matches!(last, WireMessage::Relay { .. });
    if relay_before_end || lone_relay {
        return Err(BuildError::MisplacedRelay);
    }
    Ok(())
}

fn check_tool_names(tools: &[ToolSpec]) -> Result<(), BuildError> {
    let mut seen = HashSet::new();
    for tool in tools {
        if !seen.insert(tool.name()) {
            return Err(BuildError::DuplicateToolName {
                name: tool.name().clone(),
            });
        }
    }
    Ok(())
}
