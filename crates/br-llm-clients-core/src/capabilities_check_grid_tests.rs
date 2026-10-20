use br_llm_messages::ImageMime;

use crate::capabilities_test_support::{ALL_LEVELS, ALL_MIMES, CallShape, STEPS, Step};
use crate::{Accepted, CacheRequirement, Capabilities, CapabilitiesSpec, ReasoningLevel};

fn uses_a_feature(call: &CallShape) -> bool {
    call.level != ReasoningLevel::Off
        || call.cache == CacheRequirement::Required
        || call.tools
        || call.structured
        || call.image.is_some()
}

fn every_call() -> Vec<CallShape> {
    let mut calls = Vec::new();
    for level in ALL_LEVELS {
        for cache in [CacheRequirement::Required, CacheRequirement::NotRequired] {
            for tokens in [1, 8192, u32::MAX] {
                for tools in [true, false] {
                    for structured in [true, false] {
                        let images = [None].into_iter().chain(ALL_MIMES.into_iter().map(Some));
                        for image in images {
                            calls.push(CallShape {
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
            Ok(()),
            "{call:?}"
        );
    }
}

#[test]
fn given_nothing_when_a_call_is_checked_then_it_is_refused_exactly_when_it_uses_a_feature() {
    let capabilities = Capabilities::nothing();
    for call in every_call() {
        let result = capabilities.check(&call.settings(), &call.request());
        assert_eq!(
            result.is_err(),
            uses_a_feature(&call),
            "{call:?} {result:?}"
        );
    }
}

#[test]
fn given_nothing_when_a_plain_call_asks_for_any_number_of_tokens_then_it_passes() {
    let capabilities = Capabilities::nothing();
    for tokens in [1, 8192, u32::MAX] {
        let call = CallShape::plain(tokens);
        assert_eq!(
            capabilities.check(&call.settings(), &call.request()),
            Ok(())
        );
    }
}

fn expected_refusal(spec: &CapabilitiesSpec, call: &CallShape) -> Option<Step> {
    let accepted_level = match &spec.reasoning_levels {
        Accepted::Any => true,
        Accepted::Only(list) => list.contains(&call.level),
    };
    let accepted_mime = |mime: ImageMime| match &spec.image_media_types {
        Accepted::Any => true,
        Accepted::Only(list) => list.contains(&mime),
    };
    let broken: [bool; 8] = [
        !accepted_level,
        call.cache == CacheRequirement::Required && !spec.caching,
        call.tokens > spec.max_output_tokens.get(),
        call.tools && !spec.tools,
        call.structured && !spec.structured_output,
        call.structured
            && call.tools
            && !(spec.tools && spec.structured_output && spec.structured_output_with_tools),
        call.image.is_some() && !spec.images,
        call.image
            .is_some_and(|mime| spec.images && !accepted_mime(mime)),
    ];
    STEPS
        .into_iter()
        .zip(broken)
        .find_map(|(step, is_broken)| is_broken.then_some(step))
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
        let capabilities = Capabilities::new(spec.clone()).unwrap();
        for call in every_call() {
            let result = capabilities.check(&call.settings(), &call.request());
            assert_eq!(
                result.as_ref().err().map(Step::of),
                expected_refusal(&spec, &call),
                "{spec:?} {call:?}"
            );
        }
    }
}
