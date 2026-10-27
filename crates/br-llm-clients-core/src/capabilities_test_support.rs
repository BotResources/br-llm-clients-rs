use br_llm_messages::{MediaType, ModelName, ProjectedThread};

use crate::test_support::tool;
use crate::thread_fixtures::{entry_id, human_thread, plain_images, text_block, turn_id};
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

pub(crate) const ALL_MEDIA_TYPES: [MediaType; 4] = [
    MediaType::Jpeg,
    MediaType::Png,
    MediaType::Gif,
    MediaType::Webp,
];

pub(crate) const MODEL_NAME: &str = "model-x";

pub(crate) fn settings(
    level: ReasoningLevel,
    cache: CacheRequirement,
    tokens: u32,
) -> ClientSettings {
    ClientSettings::new(
        ModelName::new(MODEL_NAME).unwrap(),
        level,
        cache,
        MaxOutputTokens::new(tokens).unwrap(),
    )
}

pub(crate) fn plain_settings() -> ClientSettings {
    settings(ReasoningLevel::Off, CacheRequirement::NotRequired, 1000)
}

pub(crate) fn shape() -> OutputMode {
    OutputMode::Structured(StructuredShape::new("answer", None, Parameters::default()).unwrap())
}

pub(crate) fn one_tool() -> Vec<ToolSpec> {
    vec![tool("search")]
}

pub(crate) fn request_of(
    history: ProjectedThread,
    tools: Vec<ToolSpec>,
    output: OutputMode,
) -> ClientRequest {
    ClientRequest::new(None, history, turn_id(), entry_id(), tools, output).unwrap()
}

pub(crate) fn hi_request(tools: Vec<ToolSpec>, output: OutputMode) -> ClientRequest {
    request_of(human_thread(vec![text_block("hi")]), tools, output)
}

pub(crate) fn plain_request() -> ClientRequest {
    hi_request(vec![], OutputMode::Plain)
}

pub(crate) fn image_request(media_types: &[MediaType]) -> ClientRequest {
    let mut content = vec![text_block("see")];
    content.extend(plain_images(media_types));
    request_of(human_thread(content), vec![], OutputMode::Plain)
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
    pub(crate) image: Option<MediaType>,
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
        let thread = match self.image {
            Some(media_type) => human_thread({
                let mut content = vec![text_block("see")];
                content.extend(plain_images(&[media_type]));
                content
            }),
            None => human_thread(vec![text_block("hello")]),
        };
        request_of(thread, tools, output)
    }
}
