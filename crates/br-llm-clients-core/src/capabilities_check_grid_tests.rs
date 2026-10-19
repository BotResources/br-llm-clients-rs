use br_llm_messages::ImageMime;

use crate::capabilities_test_support::{
    ALL_LEVELS, ALL_MIMES, one_tool, request_of, settings, shape, user_images, user_text,
};
use crate::{
    Accepted, CacheRequirement, Capabilities, CapabilitiesSpec, ClientError, ClientRequest,
    ClientSettings, OutputMode, ReasoningLevel,
};

struct Call {
    level: ReasoningLevel,
    cache: CacheRequirement,
    tokens: u32,
    tools: bool,
    structured: bool,
    image: Option<ImageMime>,
}

impl Call {
    fn settings(&self) -> ClientSettings {
        settings(self.level, self.cache, self.tokens)
    }

    fn request(&self) -> ClientRequest {
        let output = if self.structured {
            shape()
        } else {
            OutputMode::Plain
        };
        let tools = if self.tools { one_tool() } else { vec![] };
        let messages = match self.image {
            Some(mime) => vec![user_images(&[mime])],
            None => vec![user_text("hello")],
        };
        request_of(messages, tools, output)
    }

    fn uses_a_feature(&self) -> bool {
        self.level != ReasoningLevel::Off
            || self.cache == CacheRequirement::Required
            || self.tools
            || self.structured
            || self.image.is_some()
    }
}

fn every_call() -> Vec<Call> {
    let mut calls = Vec::new();
    for level in ALL_LEVELS {
        for cache in [CacheRequirement::Required, CacheRequirement::NotRequired] {
            for tokens in [1, 8192, u32::MAX] {
                for tools in [true, false] {
                    for structured in [true, false] {
                        let images = [None].into_iter().chain(ALL_MIMES.into_iter().map(Some));
                        for image in images {
                            calls.push(Call {
                                level,
                                cache,
                                tokens,
                                tools,
                                structured,
                                image,
                            });
                        }
                    }
                }
            }
        }
    }
    calls
}

#[test]
fn given_everything_when_any_call_is_checked_then_nothing_is_refused() {
    let capabilities = Capabilities::everything();
    for call in every_call() {
        assert_eq!(
            capabilities.check(&call.settings(), &call.request()),
            Ok(())
        );
    }
}

#[test]
fn given_nothing_when_a_call_is_checked_then_it_is_refused_exactly_when_it_uses_a_feature() {
    let capabilities = Capabilities::nothing();
    for call in every_call() {
        let result = capabilities.check(&call.settings(), &call.request());
        assert_eq!(result.is_err(), call.uses_a_feature(), "{:?}", result);
    }
}

#[test]
fn given_nothing_when_a_plain_call_asks_for_any_number_of_tokens_then_it_passes() {
    let capabilities = Capabilities::nothing();
    for tokens in [1, 8192, u32::MAX] {
        let call = Call {
            level: ReasoningLevel::Off,
            cache: CacheRequirement::NotRequired,
            tokens,
            tools: false,
            structured: false,
            image: None,
        };
        assert_eq!(
            capabilities.check(&call.settings(), &call.request()),
            Ok(())
        );
    }
}

fn expected_refusal(spec: &CapabilitiesSpec, call: &Call) -> Option<&'static str> {
    let accepted_level = match &spec.reasoning_levels {
        Accepted::Any => true,
        Accepted::Only(list) => list.contains(&call.level),
    };
    let accepted_mime = |mime: ImageMime| match &spec.image_media_types {
        Accepted::Any => true,
        Accepted::Only(list) => list.contains(&mime),
    };
    let rules: [(bool, &'static str); 8] = [
        (!accepted_level, "reasoning"),
        (
            call.cache == CacheRequirement::Required && !spec.caching,
            "cache",
        ),
        (call.tokens > spec.max_output_tokens.get(), "tokens"),
        (call.tools && !spec.tools, "tools"),
        (call.structured && !spec.structured_output, "structured"),
        (
            call.structured
                && call.tools
                && !(spec.tools && spec.structured_output && spec.structured_output_with_tools),
            "structured_with_tools",
        ),
        (call.image.is_some() && !spec.images, "image"),
        (
            call.image
                .is_some_and(|mime| spec.images && !accepted_mime(mime)),
            "media_type",
        ),
    ];
    rules
        .into_iter()
        .find_map(|(broken, name)| broken.then_some(name))
}

fn name_of(error: &ClientError) -> &'static str {
    match error {
        ClientError::ReasoningNotSupported { .. } => "reasoning",
        ClientError::CacheNotSupported => "cache",
        ClientError::MaxOutputTokensTooLarge { .. } => "tokens",
        ClientError::ToolsNotSupported => "tools",
        ClientError::StructuredOutputNotSupported => "structured",
        ClientError::StructuredOutputWithToolsNotSupported => "structured_with_tools",
        ClientError::ImageNotSupported => "image",
        ClientError::ImageMediaTypeNotSupported { .. } => "media_type",
        other => panic!("a case outside the check: {other:?}"),
    }
}

#[test]
fn given_a_grid_of_declarations_and_calls_when_checked_then_the_first_broken_rule_of_the_table_decides()
 {
    let specs = [
        CapabilitiesSpec::everything(),
        CapabilitiesSpec::nothing(),
        CapabilitiesSpec {
            reasoning_levels: Accepted::Only(vec![ReasoningLevel::Off, ReasoningLevel::High]),
            max_output_tokens: CapabilitiesSpec::max_output_tokens_of(
                std::num::NonZeroU32::new(8192).unwrap(),
            ),
            tools: true,
            structured_output: true,
            structured_output_with_tools: false,
            caching: false,
            images: true,
            image_media_types: Accepted::Only(vec![ImageMime::Png, ImageMime::Jpeg]),
        },
        CapabilitiesSpec {
            tools: false,
            structured_output_with_tools: true,
            ..CapabilitiesSpec::everything()
        },
        CapabilitiesSpec {
            structured_output: false,
            structured_output_with_tools: true,
            ..CapabilitiesSpec::everything()
        },
    ];
    for spec in specs {
        let capabilities = Capabilities::normalized(spec.clone());
        for call in every_call() {
            let result = capabilities.check(&call.settings(), &call.request());
            let expected = expected_refusal(capabilities.spec(), &call);
            assert_eq!(result.as_ref().err().map(name_of), expected, "{spec:?}");
        }
    }
}

#[test]
fn given_any_call_when_checked_then_the_refusal_is_never_invalid_messages() {
    for capabilities in [Capabilities::nothing(), Capabilities::everything()] {
        for call in every_call() {
            let result = capabilities.check(&call.settings(), &call.request());
            assert!(!matches!(result, Err(ClientError::InvalidMessages { .. })));
        }
    }
}
