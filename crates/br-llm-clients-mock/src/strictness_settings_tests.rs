use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    ReasoningLevel,
};

use crate::accepted::Accepted;
use crate::capabilities::Capabilities;
use crate::strictness::{CacheClass, check, check_settings, classify_cache};
use crate::test_support::*;

fn refusal(
    capabilities: &Capabilities,
    settings_value: ClientSettings,
    request_value: ClientRequest,
) -> ClientError {
    check(capabilities, &settings_value, &request_value).unwrap_err()
}

#[test]
fn given_a_level_outside_the_set_when_checked_then_reasoning_not_supported_names_it() {
    let capabilities = caps(|spec| spec.reasoning_levels = Accepted::only([ReasoningLevel::Low]));
    let error = refusal(
        &capabilities,
        settings_with(ReasoningLevel::High, CacheRequirement::NotRequired, 10),
        request(),
    );
    assert!(matches!(
        error,
        ClientError::ReasoningNotSupported { level: ReasoningLevel::High, ref reason } if !reason.is_empty()
    ));
}

#[test]
fn given_an_empty_set_of_levels_when_checked_then_every_level_is_refused_even_off() {
    let capabilities = caps(|spec| spec.reasoning_levels = Accepted::only([]));
    for level in [ReasoningLevel::Off, ReasoningLevel::Max] {
        let error = refusal(
            &capabilities,
            settings_with(level, CacheRequirement::NotRequired, 10),
            request(),
        );
        assert!(matches!(error, ClientError::ReasoningNotSupported { .. }));
    }
}

#[test]
fn given_any_level_when_checked_then_every_level_passes() {
    let capabilities = Capabilities::everything();
    for level in [
        ReasoningLevel::Off,
        ReasoningLevel::Extra,
        ReasoningLevel::Max,
    ] {
        let settings = settings_with(level, CacheRequirement::NotRequired, 10);
        assert_eq!(check(&capabilities, &settings, &request()), Ok(()));
    }
}

#[test]
fn given_required_caching_on_a_model_that_cannot_cache_when_checked_then_cache_not_supported() {
    let capabilities = caps(|spec| spec.caching = false);
    let error = refusal(
        &capabilities,
        settings_with(ReasoningLevel::Off, CacheRequirement::Required, 10),
        request(),
    );
    assert_eq!(error, ClientError::CacheNotSupported);
    let settings = settings_with(ReasoningLevel::Off, CacheRequirement::NotRequired, 10);
    assert_eq!(check(&capabilities, &settings, &request()), Ok(()));
}

#[test]
fn given_tokens_above_the_capability_when_checked_then_the_limit_is_carried() {
    let capabilities = caps(|spec| spec.max_output_tokens = MaxOutputTokens::new(100).unwrap());
    let error = refusal(
        &capabilities,
        settings_with(ReasoningLevel::Off, CacheRequirement::NotRequired, 101),
        request(),
    );
    assert_eq!(
        error,
        ClientError::MaxOutputTokensTooLarge { limit: Some(100) }
    );
    let at_limit = settings_with(ReasoningLevel::Off, CacheRequirement::NotRequired, 100);
    assert_eq!(check(&capabilities, &at_limit, &request()), Ok(()));
}

#[test]
fn given_several_failing_settings_when_checked_then_the_first_in_the_listed_order_is_returned() {
    let capabilities = caps(|spec| {
        spec.reasoning_levels = Accepted::only([]);
        spec.caching = false;
        spec.max_output_tokens = MaxOutputTokens::new(1).unwrap();
    });
    let settings = settings_with(ReasoningLevel::Low, CacheRequirement::Required, 5);
    assert!(matches!(
        check_settings(&capabilities, &settings, CacheClass::Required),
        Err(ClientError::ReasoningNotSupported { .. })
    ));
    let capabilities = caps(|spec| {
        spec.caching = false;
        spec.max_output_tokens = MaxOutputTokens::new(1).unwrap();
    });
    assert_eq!(
        check_settings(&capabilities, &settings, CacheClass::Required),
        Err(ClientError::CacheNotSupported)
    );
}

#[test]
fn given_failing_settings_and_failing_request_when_checked_then_the_settings_come_first() {
    let capabilities = caps(|spec| {
        spec.tools = false;
        spec.max_output_tokens = MaxOutputTokens::new(1).unwrap();
    });
    let bad_request = request_of(vec![user_text("hi")], vec![tool()], OutputMode::Plain);
    let error = refusal(
        &capabilities,
        settings_with(ReasoningLevel::Off, CacheRequirement::NotRequired, 5),
        bad_request,
    );
    assert!(matches!(error, ClientError::MaxOutputTokensTooLarge { .. }));
}

#[test]
fn given_a_cache_requirement_unknown_to_the_mock_when_checked_then_it_is_refused_at_the_cache_position()
 {
    let settings = settings_with(ReasoningLevel::Off, CacheRequirement::NotRequired, 5);
    let unknown =
        check_settings(&Capabilities::everything(), &settings, CacheClass::Unknown).unwrap_err();
    assert!(is_unknown_value(&unknown, "CacheRequirement"));
    let wrong_level = caps(|spec| spec.reasoning_levels = Accepted::only([]));
    assert!(matches!(
        check_settings(&wrong_level, &settings, CacheClass::Unknown),
        Err(ClientError::ReasoningNotSupported { .. })
    ));
    let too_many_tokens = caps(|spec| spec.max_output_tokens = MaxOutputTokens::new(1).unwrap());
    let big = settings_with(ReasoningLevel::Off, CacheRequirement::NotRequired, 9);
    assert!(is_unknown_value(
        &check_settings(&too_many_tokens, &big, CacheClass::Unknown).unwrap_err(),
        "CacheRequirement"
    ));
}

#[test]
fn given_the_known_cache_requirements_when_classified_then_none_is_unknown() {
    assert_eq!(
        classify_cache(CacheRequirement::Required),
        CacheClass::Required
    );
    assert_eq!(
        classify_cache(CacheRequirement::NotRequired),
        CacheClass::NotRequired
    );
}
