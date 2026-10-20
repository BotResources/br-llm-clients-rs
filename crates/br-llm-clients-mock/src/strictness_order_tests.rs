use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, ReasoningLevel,
};
use br_llm_messages::ImageMime;

use crate::call_log::Served;
use crate::strictness::check;
use crate::test_support::*;
use crate::{Accepted, Capabilities, CapabilitiesSpec};

const CAPABILITY_RULES: usize = 8;

fn breaking_settings() -> ClientSettings {
    settings_with(ReasoningLevel::Low, CacheRequirement::Required, 5)
}

fn breaking_request() -> ClientRequest {
    request_of(vec![user_image(ImageMime::Png)], vec![tool()], shape())
}

fn invalid_messages_request() -> ClientRequest {
    request_of(
        vec![assistant_text("first"), user_image(ImageMime::Png)],
        vec![tool()],
        shape(),
    )
}

fn capabilities_breaking_from(first: usize) -> Capabilities {
    let mut spec = CapabilitiesSpec::everything();
    if first == 0 {
        spec.reasoning_levels = Accepted::Only(vec![]);
    }
    if first <= 1 {
        spec.caching = false;
    }
    if first <= 2 {
        spec.max_output_tokens = MaxOutputTokens::new(1).unwrap();
    }
    if first <= 3 {
        spec.tools = false;
    }
    if first <= 4 {
        spec.structured_output = false;
    }
    if first <= 5 {
        spec.structured_output_with_tools = false;
    }
    if first <= 6 {
        spec.images = false;
    }
    if first == 7 {
        spec.image_media_types = Accepted::Only(vec![ImageMime::Gif]);
    }
    Capabilities::new(spec).unwrap()
}

fn expected_refusal(rule: usize) -> ClientError {
    match rule {
        0 => ClientError::ReasoningNotSupported {
            level: ReasoningLevel::Low,
            reason: String::new(),
        },
        1 => ClientError::CacheNotSupported,
        2 => ClientError::MaxOutputTokensTooLarge { limit: Some(1) },
        3 => ClientError::ToolsNotSupported,
        4 => ClientError::StructuredOutputNotSupported,
        5 => ClientError::StructuredOutputWithToolsNotSupported,
        6 => ClientError::ImageNotSupported,
        _ => ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Png,
        },
    }
}

fn same_case(left: &ClientError, right: &ClientError) -> bool {
    match (left, right) {
        (
            ClientError::ReasoningNotSupported { level: a, .. },
            ClientError::ReasoningNotSupported { level: b, .. },
        ) => a == b,
        _ => left == right,
    }
}

#[test]
fn given_a_request_breaking_capability_rules_when_checked_then_the_earliest_listed_rule_is_given() {
    for first in 0..CAPABILITY_RULES {
        let capabilities = capabilities_breaking_from(first);
        let error = check(&capabilities, &breaking_settings(), &breaking_request()).unwrap_err();
        assert!(
            same_case(&error, &expected_refusal(first)),
            "rule {first} gave {error:?}"
        );
    }
}

#[test]
fn given_invalid_messages_when_any_capability_rule_is_also_broken_then_invalid_messages_is_given() {
    let invalid = invalid_messages_request();
    let expected = invalid.check_messages().unwrap_err();
    assert!(matches!(expected, ClientError::InvalidMessages { .. }));
    for first in 0..CAPABILITY_RULES {
        let capabilities = capabilities_breaking_from(first);
        assert_eq!(
            check(&capabilities, &breaking_settings(), &invalid),
            Err(expected.clone()),
            "rule {first}"
        );
    }
}

#[test]
fn given_invalid_messages_when_the_level_the_cache_or_the_tokens_are_broken_then_invalid_messages()
{
    let invalid = invalid_messages_request();
    let cases = [
        (
            caps(|spec| spec.reasoning_levels = Accepted::Only(vec![ReasoningLevel::Off])),
            settings_with(ReasoningLevel::High, CacheRequirement::NotRequired, 5),
        ),
        (
            caps(|spec| spec.caching = false),
            settings_with(ReasoningLevel::Low, CacheRequirement::Required, 5),
        ),
        (
            caps(|spec| spec.max_output_tokens = MaxOutputTokens::new(1).unwrap()),
            settings_with(ReasoningLevel::Low, CacheRequirement::NotRequired, 5),
        ),
    ];
    for (capabilities, settings_value) in cases {
        assert!(matches!(
            check(&capabilities, &settings_value, &invalid),
            Err(ClientError::InvalidMessages { .. })
        ));
    }
}

#[test]
fn given_an_empty_set_of_levels_when_the_messages_differ_then_only_valid_messages_give_reasoning() {
    let capabilities = caps(|spec| spec.reasoning_levels = Accepted::Only(vec![]));
    for level in [ReasoningLevel::Off, ReasoningLevel::Max] {
        let settings_value = settings_with(level, CacheRequirement::NotRequired, 5);
        assert!(matches!(
            check(&capabilities, &settings_value, &request()),
            Err(ClientError::ReasoningNotSupported { level: given, .. }) if given == level
        ));
        assert!(matches!(
            check(&capabilities, &settings_value, &invalid_messages_request()),
            Err(ClientError::InvalidMessages { .. })
        ));
    }
}

#[test]
fn given_a_refused_level_when_checked_then_the_reason_names_the_model_id_and_not_the_mock() {
    let capabilities = caps(|spec| spec.reasoning_levels = Accepted::Only(vec![]));
    let settings_value = ClientSettings::new(
        model("model-in-the-settings"),
        ReasoningLevel::High,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(5).unwrap(),
    );
    let Err(ClientError::ReasoningNotSupported { level, reason }) =
        check(&capabilities, &settings_value, &request())
    else {
        panic!("the level must be refused");
    };
    assert_eq!(level, ReasoningLevel::High);
    assert!(reason.contains("model-in-the-settings"));
    assert!(!reason.to_lowercase().contains("mock"));
}

#[test]
fn given_a_call_with_invalid_messages_and_a_broken_level_when_made_then_the_log_shows_invalid_messages()
 {
    let client = crate::MockClient::new(settings(), capabilities_breaking_from(0));
    let (outcome, sink) = run(&client, &invalid_messages_request());
    assert!(matches!(
        error_of(&outcome),
        ClientError::InvalidMessages { .. }
    ));
    assert!(sink.is_empty());
    assert_eq!(outcome.report().total_duration(), std::time::Duration::ZERO);
    assert!(matches!(
        client.log()[0].served(),
        Served::Refusal(ClientError::InvalidMessages { .. })
    ));
}
