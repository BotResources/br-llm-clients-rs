use br_llm_clients_core::{CacheRequirement, ClientError, MaxOutputTokens, ReasoningLevel};
use br_llm_messages::ImageMime;

use crate::strictness::check;
use crate::test_support::*;
use crate::{Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec};

fn level_check(capabilities: &Capabilities, level: ReasoningLevel) -> Result<(), ClientError> {
    let settings_value = settings_with(level, CacheRequirement::NotRequired, 10);
    check(capabilities, &settings_value, &request())
}

fn image_check(capabilities: &Capabilities, mime: ImageMime) -> Result<(), ClientError> {
    let with_image = request_of(
        vec![user_image(mime)],
        vec![],
        br_llm_clients_core::OutputMode::Plain,
    );
    check(capabilities, &settings(), &with_image)
}

#[test]
fn given_everything_when_built_then_it_accepts_any_level_the_largest_tokens_and_every_feature() {
    let capabilities = everything_caps();
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
    assert!(capabilities.structured_output());
    assert!(capabilities.images());
    assert_eq!(capabilities.reasoning_levels(), &Accepted::Any);
}

#[test]
fn given_images_with_an_empty_set_of_media_types_when_built_then_inconsistent() {
    let spec = CapabilitiesSpec {
        images: true,
        image_media_types: Accepted::Only(vec![]),
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
        image_media_types: Accepted::Only(vec![]),
        ..CapabilitiesSpec::everything()
    };
    let capabilities = Capabilities::new(spec).unwrap();
    assert_eq!(
        image_check(&capabilities, ImageMime::Png),
        Err(ClientError::ImageNotSupported)
    );
}

#[test]
fn given_an_empty_set_of_levels_when_built_then_it_is_legal() {
    let spec = CapabilitiesSpec {
        reasoning_levels: Accepted::Only(vec![]),
        ..CapabilitiesSpec::everything()
    };
    assert!(Capabilities::new(spec).is_ok());
}

#[test]
fn given_a_set_holding_off_when_a_call_is_checked_then_off_is_a_member_like_any_other() {
    let only_off = caps(|spec| spec.reasoning_levels = Accepted::Only(vec![ReasoningLevel::Off]));
    assert_eq!(level_check(&only_off, ReasoningLevel::Off), Ok(()));
    assert!(matches!(
        level_check(&only_off, ReasoningLevel::Low),
        Err(ClientError::ReasoningNotSupported {
            level: ReasoningLevel::Low,
            ..
        })
    ));
    let without_off =
        caps(|spec| spec.reasoning_levels = Accepted::Only(vec![ReasoningLevel::Low]));
    assert!(matches!(
        level_check(&without_off, ReasoningLevel::Off),
        Err(ClientError::ReasoningNotSupported {
            level: ReasoningLevel::Off,
            ..
        })
    ));
}

#[test]
fn given_any_when_a_call_is_checked_then_every_level_and_media_type_is_accepted() {
    let capabilities = everything_caps();
    for level in [
        ReasoningLevel::Off,
        ReasoningLevel::Low,
        ReasoningLevel::Medium,
        ReasoningLevel::High,
        ReasoningLevel::Extra,
        ReasoningLevel::Max,
    ] {
        assert_eq!(level_check(&capabilities, level), Ok(()));
    }
    assert_eq!(image_check(&capabilities, ImageMime::Webp), Ok(()));
}

#[test]
fn given_a_set_of_media_types_when_a_call_is_checked_then_only_its_members_are_accepted() {
    let capabilities = caps(|spec| {
        spec.image_media_types = Accepted::Only(vec![ImageMime::Png, ImageMime::Jpeg]);
    });
    assert_eq!(image_check(&capabilities, ImageMime::Png), Ok(()));
    assert_eq!(
        image_check(&capabilities, ImageMime::Gif),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Gif
        })
    );
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
    assert!(everything_caps().accepts_structured_output_with_tools());
}
