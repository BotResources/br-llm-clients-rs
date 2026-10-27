use br_llm_clients_core::{
    CacheRequirement, ClientError, MaxOutputTokens, OutputMode, ReasoningLevel,
};
use br_llm_messages::MediaType;

use crate::strictness::check;
use crate::test_support::*;
use crate::{Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec};

fn level_check(capabilities: &Capabilities, level: ReasoningLevel) -> Result<(), ClientError> {
    let settings_value = settings_with(level, CacheRequirement::NotRequired, 10);
    check(capabilities, &settings_value, &request())
}

fn image_check(capabilities: &Capabilities, media_type: MediaType) -> Result<(), ClientError> {
    let with_image = request_of(
        vec![
            crate::fixtures::text_block("look"),
            crate::fixtures::plain_image(media_type),
        ],
        vec![],
        OutputMode::Plain,
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
    let capabilities = caps(|spec| {
        spec.tools = false;
        spec.max_output_tokens = MaxOutputTokens::new(10).unwrap();
    });
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
    let capabilities = caps(|spec| {
        spec.images = false;
        spec.image_media_types = Accepted::Only(vec![]);
    });
    assert_eq!(
        image_check(&capabilities, MediaType::Png),
        Err(ClientError::ImageNotSupported)
    );
}

#[test]
fn given_an_empty_set_of_levels_when_built_then_it_is_legal_and_refuses_every_level() {
    let capabilities = caps(|spec| spec.reasoning_levels = Accepted::Only(vec![]));
    for level in [ReasoningLevel::Off, ReasoningLevel::Max] {
        assert!(matches!(
            level_check(&capabilities, level),
            Err(ClientError::ReasoningNotSupported { .. })
        ));
    }
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
    for media_type in [
        MediaType::Png,
        MediaType::Jpeg,
        MediaType::Gif,
        MediaType::Webp,
    ] {
        assert_eq!(image_check(&capabilities, media_type), Ok(()));
    }
}

#[test]
fn given_a_set_of_media_types_when_a_call_is_checked_then_only_its_members_are_accepted() {
    let capabilities = caps(|spec| {
        spec.image_media_types = Accepted::Only(vec![MediaType::Png, MediaType::Jpeg]);
    });
    assert_eq!(image_check(&capabilities, MediaType::Png), Ok(()));
    assert_eq!(
        image_check(&capabilities, MediaType::Gif),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: MediaType::Gif
        })
    );
}

#[test]
fn given_tools_or_structured_output_missing_when_asked_then_the_combination_field_is_ignored() {
    for (tools, structured) in [(false, true), (true, false), (false, false)] {
        let capabilities = caps(|spec| {
            spec.tools = tools;
            spec.structured_output = structured;
            spec.structured_output_with_tools = true;
        });
        assert!(!capabilities.accepts_structured_output_with_tools());
    }
    assert!(everything_caps().accepts_structured_output_with_tools());
}

#[test]
fn given_the_combination_field_off_when_tools_and_structured_are_on_then_the_combined_request_is_refused()
 {
    let capabilities = caps(|spec| spec.structured_output_with_tools = false);
    let combined = request_of(
        vec![crate::fixtures::text_block("ask")],
        vec![tool()],
        shape(),
    );
    assert_eq!(
        check(&capabilities, &settings(), &combined),
        Err(ClientError::StructuredOutputWithToolsNotSupported)
    );
    let ignored = caps(|spec| {
        spec.tools = false;
        spec.structured_output_with_tools = false;
    });
    assert_eq!(
        check(&ignored, &settings(), &combined),
        Err(ClientError::ToolsNotSupported)
    );
}
