use br_llm_messages::{
    AssistantBlock, Base64Data, Image, ImageMime, ModelId, ToolCallId, ToolName, ToolResult,
    ToolResultBlock, WireMessage, WireUserBlock,
};

use crate::test_support::{text, tool};
use crate::{
    CacheRequirement, Capabilities, CapabilitiesSpec, ClientError, ClientRequest, ClientSettings,
    MaxOutputTokens, OutputMode, Parameters, ReasoningLevel, StructuredShape, ToolSpec,
};

pub(crate) const ALL_LEVELS: [ReasoningLevel; 6] = [
    ReasoningLevel::Off,
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

fn position_of(level: ReasoningLevel) -> usize {
    match level {
        ReasoningLevel::Off => 0,
        ReasoningLevel::Low => 1,
        ReasoningLevel::Medium => 2,
        ReasoningLevel::High => 3,
        ReasoningLevel::Extra => 4,
        ReasoningLevel::Max => 5,
    }
}

#[test]
fn given_every_reasoning_level_when_listed_then_all_levels_holds_it_at_its_position() {
    for (index, level) in ALL_LEVELS.iter().enumerate() {
        assert_eq!(position_of(*level), index);
    }
}

pub(crate) const ALL_MIMES: [ImageMime; 4] = [
    ImageMime::Jpeg,
    ImageMime::Png,
    ImageMime::Gif,
    ImageMime::Webp,
];

pub(crate) const MODEL_NAME: &str = "model-x";

pub(crate) fn settings(
    level: ReasoningLevel,
    cache: CacheRequirement,
    tokens: u32,
) -> ClientSettings {
    ClientSettings::new(
        ModelId::new(MODEL_NAME).unwrap(),
        level,
        cache,
        MaxOutputTokens::new(tokens).unwrap(),
    )
}

pub(crate) fn plain_settings() -> ClientSettings {
    settings(ReasoningLevel::Off, CacheRequirement::NotRequired, 1000)
}

pub(crate) fn user_text(value: &str) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::Text { text: text(value) }],
    }
}

pub(crate) fn assistant_text(value: &str) -> WireMessage {
    WireMessage::Assistant {
        content: vec![AssistantBlock::Text { text: text(value) }],
    }
}

pub(crate) fn image_block(mime: ImageMime) -> WireUserBlock {
    WireUserBlock::Image(Image {
        mime,
        data: Base64Data::new("aGVsbG8=").unwrap(),
    })
}

pub(crate) fn user_images(mimes: &[ImageMime]) -> WireMessage {
    WireMessage::User {
        content: mimes.iter().map(|mime| image_block(*mime)).collect(),
    }
}

pub(crate) fn user_result_message(id: &str) -> WireMessage {
    WireMessage::User {
        content: vec![WireUserBlock::ToolResult(ToolResult::new(
            ToolCallId::new(id).unwrap(),
            ToolName::new("search").unwrap(),
            vec![ToolResultBlock::text(text("found"))],
            false,
        ))],
    }
}

pub(crate) fn shape() -> OutputMode {
    OutputMode::Structured(StructuredShape::new("answer", None, Parameters::default()).unwrap())
}

pub(crate) fn one_tool() -> Vec<ToolSpec> {
    vec![tool("search")]
}

pub(crate) fn request_of(
    messages: Vec<WireMessage>,
    tools: Vec<ToolSpec>,
    output: OutputMode,
) -> ClientRequest {
    ClientRequest::new(None, messages, tools, output).unwrap()
}

pub(crate) fn plain_request() -> ClientRequest {
    request_of(vec![user_text("hello")], vec![], OutputMode::Plain)
}

pub(crate) fn image_request(mimes: &[ImageMime]) -> ClientRequest {
    request_of(vec![user_images(mimes)], vec![], OutputMode::Plain)
}

pub(crate) fn caps_with(change: impl FnOnce(&mut CapabilitiesSpec)) -> Capabilities {
    let mut spec = CapabilitiesSpec::everything();
    change(&mut spec);
    Capabilities::normalized(spec)
}

pub(crate) trait AmbiguousIfDefault<Marker> {
    fn proof() {}
}
impl<T: ?Sized> AmbiguousIfDefault<()> for T {}
impl<T: Default> AmbiguousIfDefault<u8> for T {}

pub(crate) fn assert_plain_data<T: Send + Sync + 'static + Eq + Clone + std::fmt::Debug>() {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Step {
    Reasoning,
    Cache,
    Tokens,
    Tools,
    Structured,
    StructuredWithTools,
    Image,
    MediaType,
}

pub(crate) const STEPS: [Step; 8] = [
    Step::Reasoning,
    Step::Cache,
    Step::Tokens,
    Step::Tools,
    Step::Structured,
    Step::StructuredWithTools,
    Step::Image,
    Step::MediaType,
];

impl Step {
    pub(crate) fn of(error: &ClientError) -> Self {
        match error {
            ClientError::ReasoningNotSupported { .. } => Self::Reasoning,
            ClientError::CacheNotSupported => Self::Cache,
            ClientError::MaxOutputTokensTooLarge { .. } => Self::Tokens,
            ClientError::ToolsNotSupported => Self::Tools,
            ClientError::StructuredOutputNotSupported => Self::Structured,
            ClientError::StructuredOutputWithToolsNotSupported => Self::StructuredWithTools,
            ClientError::ImageNotSupported => Self::Image,
            ClientError::ImageMediaTypeNotSupported { .. } => Self::MediaType,
            other => panic!("a case outside the check: {other:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CallShape {
    pub(crate) level: ReasoningLevel,
    pub(crate) cache: CacheRequirement,
    pub(crate) tokens: u32,
    pub(crate) tools: bool,
    pub(crate) structured: bool,
    pub(crate) image: Option<ImageMime>,
}

impl CallShape {
    pub(crate) fn plain(tokens: u32) -> Self {
        Self {
            level: ReasoningLevel::Off,
            cache: CacheRequirement::NotRequired,
            tokens,
            tools: false,
            structured: false,
            image: None,
        }
    }

    pub(crate) fn settings(&self) -> ClientSettings {
        settings(self.level, self.cache, self.tokens)
    }

    pub(crate) fn request(&self) -> ClientRequest {
        let output = if self.structured {
            shape()
        } else {
            OutputMode::Plain
        };
        let tools = if self.tools { one_tool() } else { vec![] };
        let messages = match self.image {
            Some(mime) => vec![user_text("see"), user_images(&[mime])],
            None => vec![user_text("hello")],
        };
        request_of(messages, tools, output)
    }
}
