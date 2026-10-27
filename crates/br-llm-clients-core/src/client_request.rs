use std::collections::HashSet;

use br_llm_messages::{EntryId, PlainText, ProjectedThread, TurnId};

use crate::build_error::BuildError;
use crate::output_mode::OutputMode;
use crate::tool::ToolSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct ClientRequest {
    system: Option<PlainText>,
    history: ProjectedThread,
    reply_turn: TurnId,
    reply_entry: EntryId,
    tools: Vec<ToolSpec>,
    output: OutputMode,
}

impl ClientRequest {
    pub fn new(
        system: Option<PlainText>,
        history: ProjectedThread,
        reply_turn: TurnId,
        reply_entry: EntryId,
        tools: Vec<ToolSpec>,
        output: OutputMode,
    ) -> Result<Self, BuildError> {
        if history.turns().is_empty() {
            return Err(BuildError::EmptyThread);
        }
        check_tool_names(&tools)?;
        Ok(Self {
            system,
            history,
            reply_turn,
            reply_entry,
            tools,
            output,
        })
    }

    pub fn system(&self) -> Option<&PlainText> {
        self.system.as_ref()
    }

    pub fn history(&self) -> &ProjectedThread {
        &self.history
    }

    pub fn reply_turn(&self) -> TurnId {
        self.reply_turn
    }

    pub fn reply_entry(&self) -> EntryId {
        self.reply_entry
    }

    pub fn tools(&self) -> &[ToolSpec] {
        &self.tools
    }

    pub fn output(&self) -> &OutputMode {
        &self.output
    }
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
