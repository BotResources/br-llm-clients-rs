use br_llm_messages::{
    AssistantBlock, Base64Data, Image, ImageMime, ModelId, ToolCallId, ToolName, ToolResult,
    ToolResultBlock, WireMessage, WireUserBlock,
};

use crate::test_support::{text, tool};
use crate::{
    CacheRequirement, Capabilities, CapabilitiesSpec, ClientRequest, ClientSettings,
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
