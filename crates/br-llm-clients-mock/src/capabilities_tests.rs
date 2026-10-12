use br_llm_clients_core::{MaxOutputTokens, ReasoningLevel};
use br_llm_messages::ImageMime;

use crate::accepted::Accepted;
use crate::capabilities::{Capabilities, CapabilitiesSpec};
use crate::capabilities_error::CapabilitiesError;

#[test]
fn given_everything_when_built_then_it_accepts_any_level_the_largest_tokens_and_every_feature() {
    let capabilities = Capabilities::everything();
    let spec = capabilities.spec();
    assert_eq!(spec.reasoning_levels, Accepted::Any);
    assert_eq!(spec.max_output_tokens.get(), u32::MAX);
    assert!(spec.tools);
    assert!(spec.structured_output);
    assert!(spec.structured_output_with_tools);
    assert!(spec.caching);
    assert!(spec.images);
    assert_eq!(spec.image_media_types, Accepted::Any);
}

#[test]
fn given_the_everything_spec_when_built_then_it_equals_the_everything_capabilities() {
    assert_eq!(
        Capabilities::new(CapabilitiesSpec::everything()).unwrap(),
        Capabilities::everything()
    );
}

#[test]
fn given_everything_with_changed_fields_when_built_then_only_those_fields_differ() {
    let spec = CapabilitiesSpec {
        tools: false,
        max_output_tokens: MaxOutputTokens::new(10).unwrap(),
        ..CapabilitiesSpec::everything()
    };
    let capabilities = Capabilities::new(spec).unwrap();
    assert!(!capabilities.tools());
    assert_eq!(capabilities.max_output_tokens().get(), 10);
    assert!(capabilities.caching());
}

#[test]
fn given_images_with_an_empty_set_of_media_types_when_built_then_inconsistent() {
    let spec = CapabilitiesSpec {
        images: true,
        image_media_types: Accepted::only([]),
        ..CapabilitiesSpec::everything()
    };
    assert_eq!(
        Capabilities::new(spec),
        Err(CapabilitiesError::InconsistentCapabilities)
    );
}

#[test]
fn given_no_images_with_an_empty_set_of_media_types_when_built_then_the_set_is_ignored() {
    let spec = CapabilitiesSpec {
        images: false,
        image_media_types: Accepted::only([]),
        ..CapabilitiesSpec::everything()
    };
    assert!(Capabilities::new(spec).is_ok());
}

#[test]
fn given_an_empty_set_of_levels_when_built_then_it_is_legal() {
    let spec = CapabilitiesSpec {
        reasoning_levels: Accepted::only([]),
        ..CapabilitiesSpec::everything()
    };
    assert!(Capabilities::new(spec).is_ok());
}

#[test]
fn given_a_set_holding_off_when_asked_then_off_is_a_member_like_any_other() {
    let only_off = Accepted::only([ReasoningLevel::Off]);
    assert!(only_off.accepts(&ReasoningLevel::Off));
    assert!(!only_off.accepts(&ReasoningLevel::Low));
    let without_off = Accepted::only([ReasoningLevel::Low]);
    assert!(!without_off.accepts(&ReasoningLevel::Off));
}

#[test]
fn given_any_when_asked_then_every_value_is_accepted() {
    for level in [
        ReasoningLevel::Off,
        ReasoningLevel::Low,
        ReasoningLevel::Medium,
        ReasoningLevel::High,
        ReasoningLevel::Extra,
        ReasoningLevel::Max,
    ] {
        assert!(Accepted::Any.accepts(&level));
    }
    assert!(Accepted::<ImageMime>::Any.accepts(&ImageMime::Webp));
}

#[test]
fn given_a_set_of_media_types_when_asked_then_only_its_members_are_accepted() {
    let set = Accepted::only([ImageMime::Png, ImageMime::Jpeg]);
    assert!(set.accepts(&ImageMime::Png));
    assert!(!set.accepts(&ImageMime::Gif));
}

#[test]
fn given_tools_or_structured_output_missing_when_asked_then_the_combination_field_is_ignored() {
    for (tools, structured) in [(false, true), (true, false), (false, false)] {
        let capabilities = Capabilities::new(CapabilitiesSpec {
            tools,
            structured_output: structured,
            structured_output_with_tools: true,
            ..CapabilitiesSpec::everything()
        })
        .unwrap();
        assert!(!capabilities.accepts_structured_output_with_tools());
    }
    assert!(Capabilities::everything().accepts_structured_output_with_tools());
}
