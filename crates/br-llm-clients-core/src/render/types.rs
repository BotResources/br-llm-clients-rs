use br_llm_messages::{ContentBlock, ImageBlock, RuntimeToolCall, ToolName};

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedHistory {
    messages: Vec<RenderedMessage>,
}

impl RenderedHistory {
    pub(crate) fn new(messages: Vec<RenderedMessage>) -> Self {
        Self { messages }
    }

    pub fn messages(&self) -> &[RenderedMessage] {
        &self.messages
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedMessage {
    role: RenderedRole,
    parts: Vec<RenderedPart>,
}

impl RenderedMessage {
    pub(crate) fn new(role: RenderedRole, parts: Vec<RenderedPart>) -> Self {
        Self { role, parts }
    }

    pub fn role(&self) -> RenderedRole {
        self.role
    }

    pub fn parts(&self) -> &[RenderedPart] {
        &self.parts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderedRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RenderedPart {
    Text(String),
    Image(ImageBlock),
    ToolCall {
        call: RuntimeToolCall,
        wire_id: String,
    },
    ToolResult {
        wire_id: String,
        tool_name: ToolName,
        is_error: bool,
        parts: Vec<RenderedResultPart>,
    },
    Block(ContentBlock),
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RenderedResultPart {
    Text(String),
    Image(ImageBlock),
}
