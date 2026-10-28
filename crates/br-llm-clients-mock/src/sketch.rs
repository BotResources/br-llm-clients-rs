use br_llm_messages::{ModelResponse, ReplayMaterial, ResponseEnding, Usage};

use crate::build_error::MockBuildError;
use crate::json_input::JsonInput;
use crate::sketch_build;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    Text(String),
    Thinking {
        text: String,
        replay: Option<ReplayMaterial>,
    },
    ToolCall {
        name: String,
        external_id: Option<String>,
        arguments: JsonInput,
    },
    Structured(JsonInput),
}

#[derive(Debug, Clone, Default, PartialEq)]
#[must_use = "a sketch does nothing until it is given to a reply"]
pub struct Sketch {
    pub(crate) items: Vec<Item>,
    pub(crate) ending: Option<ResponseEnding>,
    pub(crate) usage: Option<Usage>,
    pub(crate) model: Option<String>,
    pub(crate) provider_response_id: Option<String>,
}

impl Sketch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.items.push(Item::Text(text.into()));
        self
    }

    pub fn thinking(mut self, text: impl Into<String>) -> Self {
        self.items.push(Item::Thinking {
            text: text.into(),
            replay: None,
        });
        self
    }

    pub fn thinking_with_replay(mut self, text: impl Into<String>, replay: ReplayMaterial) -> Self {
        self.items.push(Item::Thinking {
            text: text.into(),
            replay: Some(replay),
        });
        self
    }

    pub fn tool_call(mut self, name: impl Into<String>, arguments: impl Into<JsonInput>) -> Self {
        self.items.push(Item::ToolCall {
            name: name.into(),
            external_id: None,
            arguments: arguments.into(),
        });
        self
    }

    pub fn tool_call_with_external_id(
        mut self,
        name: impl Into<String>,
        external_id: impl Into<String>,
        arguments: impl Into<JsonInput>,
    ) -> Self {
        self.items.push(Item::ToolCall {
            name: name.into(),
            external_id: Some(external_id.into()),
            arguments: arguments.into(),
        });
        self
    }

    pub fn structured(mut self, value: impl Into<JsonInput>) -> Self {
        self.items.push(Item::Structured(value.into()));
        self
    }

    pub fn ending(mut self, ending: ResponseEnding) -> Self {
        self.ending = Some(ending);
        self
    }

    pub fn usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn provider_response_id(mut self, id: impl Into<String>) -> Self {
        self.provider_response_id = Some(id.into());
        self
    }

    pub fn to_response(&self) -> Result<ModelResponse, MockBuildError> {
        sketch_build::build(self).map_err(|error| MockBuildError::SketchInvalid {
            reason: error.to_string(),
        })
    }
}
