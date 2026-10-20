use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    ReasoningLevel,
};

use crate::strictness::check;
use crate::test_support::*;
use crate::{Accepted, Capabilities};

fn refusal(
    capabilities: &Capabilities,
    settings_value: ClientSettings,
    request_value: ClientRequest,
) -> ClientError {
    check(capabilities, &settings_value, &request_value).unwrap_err()
}

#[test]
fn given_a_level_outside_the_set_when_checked_then_reasoning_not_supported_names_it() {
    let capabilities =
        caps(|spec| spec.reasoning_levels = Accepted::Only(vec![ReasoningLevel::Low]));
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
fn given_an_empty_set_of_levels_when_the_messages_pass_then_every_level_is_refused_even_off() {
    let capabilities = caps(|spec| spec.reasoning_levels = Accepted::Only(vec![]));
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
    let capabilities = everything_caps();
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
        spec.reasoning_levels = Accepted::Only(vec![]);
        spec.caching = false;
        spec.max_output_tokens = MaxOutputTokens::new(1).unwrap();
    });
    let settings = settings_with(ReasoningLevel::Low, CacheRequirement::Required, 5);
    assert!(matches!(
        check(&capabilities, &settings, &request()),
        Err(ClientError::ReasoningNotSupported { .. })
    ));
    let capabilities = caps(|spec| {
        spec.caching = false;
        spec.max_output_tokens = MaxOutputTokens::new(1).unwrap();
    });
    assert_eq!(
        check(&capabilities, &settings, &request()),
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
