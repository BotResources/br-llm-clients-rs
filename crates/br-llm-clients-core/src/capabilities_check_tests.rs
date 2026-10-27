use br_llm_messages::{ContentBlock, MediaType, ResponseEnding};

use crate::capabilities_test_support::{
    ALL_LEVELS, MODEL_NAME, caps_with, hi_request, image_request, one_tool, plain_request,
    plain_settings, request_of, settings, shape,
};
use crate::thread_fixtures::{
    Fixture, ai_turn, human_turn, response, text_block, tool_call, turn_id,
};
use crate::{
    Accepted, CacheRequirement, Capabilities, CapabilitiesSpec, ClientError, OutputMode,
    ReasoningLevel,
};

fn limit_spec(limit: u32) -> CapabilitiesSpec {
    CapabilitiesSpec {
        max_output_tokens: CapabilitiesSpec::max_output_tokens_of(
            std::num::NonZeroU32::new(limit).unwrap(),
        ),
        ..CapabilitiesSpec::everything()
    }
}

fn level_settings(level: ReasoningLevel) -> crate::ClientSettings {
    settings(level, CacheRequirement::NotRequired, 1000)
}

#[test]
fn given_a_level_that_only_does_not_list_when_checked_then_reasoning_not_supported_names_the_level_and_the_exact_reason()
 {
    let capabilities = caps_with(|spec| {
        spec.reasoning_levels = Accepted::Only(vec![ReasoningLevel::Low]);
    });
    assert_eq!(
        capabilities.check(&level_settings(ReasoningLevel::High), &plain_request()),
        Err(ClientError::ReasoningNotSupported {
            level: ReasoningLevel::High,
            reason: format!(
                "the declared capabilities of model {MODEL_NAME} do not accept this level"
            ),
        })
    );
}

#[test]
fn given_the_reason_when_read_then_it_carries_the_model_and_not_the_level() {
    let capabilities = caps_with(|spec| spec.reasoning_levels = Accepted::Only(Vec::new()));
    let Err(ClientError::ReasoningNotSupported { reason, .. }) =
        capabilities.check(&level_settings(ReasoningLevel::Medium), &plain_request())
    else {
        panic!("expected ReasoningNotSupported");
    };
    assert_eq!(
        reason,
        "the declared capabilities of model model-x do not accept this level"
    );
    assert!(!reason.to_lowercase().contains("medium"));
}

#[test]
fn given_off_when_the_list_does_not_hold_it_then_off_is_refused_like_any_other_level() {
    let capabilities = caps_with(|spec| {
        spec.reasoning_levels = Accepted::Only(vec![ReasoningLevel::Low]);
    });
    assert!(matches!(
        capabilities.check(&level_settings(ReasoningLevel::Off), &plain_request()),
        Err(ClientError::ReasoningNotSupported {
            level: ReasoningLevel::Off,
            ..
        })
    ));
}

#[test]
fn given_the_four_forms_of_levels_when_checked_for_every_level_then_the_declaration_alone_decides()
{
    let forms = [
        (Accepted::Any, ALL_LEVELS.to_vec()),
        (Accepted::Only(Vec::new()), Vec::new()),
        (
            Accepted::Only(vec![ReasoningLevel::Off]),
            vec![ReasoningLevel::Off],
        ),
        (
            Accepted::Only(vec![ReasoningLevel::Low, ReasoningLevel::Max]),
            vec![ReasoningLevel::Low, ReasoningLevel::Max],
        ),
    ];
    for (form, accepted) in forms {
        let capabilities = caps_with(|spec| spec.reasoning_levels = form.clone());
        for level in ALL_LEVELS {
            let result = capabilities.check(&level_settings(level), &plain_request());
            assert_eq!(
                result.is_ok(),
                accepted.contains(&level),
                "{form:?} {level:?}"
            );
        }
    }
}

#[test]
fn given_required_caching_when_the_model_cannot_cache_then_cache_not_supported() {
    let capabilities = caps_with(|spec| spec.caching = false);
    let required = settings(ReasoningLevel::Off, CacheRequirement::Required, 1000);
    assert_eq!(
        capabilities.check(&required, &plain_request()),
        Err(ClientError::CacheNotSupported)
    );
}

#[test]
fn given_the_cache_requirements_when_checked_against_both_caching_values_then_only_required_without_caching_is_refused()
 {
    for caching in [true, false] {
        let capabilities = caps_with(|spec| spec.caching = caching);
        for requirement in [CacheRequirement::Required, CacheRequirement::NotRequired] {
            let result = capabilities.check(
                &settings(ReasoningLevel::Off, requirement, 1000),
                &plain_request(),
            );
            let refused = requirement == CacheRequirement::Required && !caching;
            assert_eq!(result.is_err(), refused, "{caching} {requirement:?}");
        }
    }
}

#[test]
fn given_a_limit_when_the_tokens_are_above_it_then_the_limit_is_always_given() {
    let capabilities = Capabilities::normalized(limit_spec(100));
    for (tokens, limit) in [(101, 100), (1000, 100), (u32::MAX, 100)] {
        assert_eq!(
            capabilities.check(
                &settings(ReasoningLevel::Off, CacheRequirement::NotRequired, tokens),
                &plain_request()
            ),
            Err(ClientError::MaxOutputTokensTooLarge { limit: Some(limit) })
        );
    }
}

#[test]
fn given_a_limit_when_the_tokens_are_equal_or_below_it_then_the_call_passes() {
    let capabilities = Capabilities::normalized(limit_spec(100));
    for tokens in [1, 99, 100] {
        assert_eq!(
            capabilities.check(
                &settings(ReasoningLevel::Off, CacheRequirement::NotRequired, tokens),
                &plain_request()
            ),
            Ok(())
        );
    }
}

#[test]
fn given_the_smallest_and_largest_limits_when_checked_then_the_edges_hold() {
    let smallest = Capabilities::normalized(limit_spec(1));
    let at = settings(ReasoningLevel::Off, CacheRequirement::NotRequired, 1);
    let above = settings(ReasoningLevel::Off, CacheRequirement::NotRequired, 2);
    assert_eq!(smallest.check(&at, &plain_request()), Ok(()));
    assert_eq!(
        smallest.check(&above, &plain_request()),
        Err(ClientError::MaxOutputTokensTooLarge { limit: Some(1) })
    );
    let largest = Capabilities::normalized(limit_spec(u32::MAX));
    let top = settings(ReasoningLevel::Off, CacheRequirement::NotRequired, u32::MAX);
    assert_eq!(largest.check(&top, &plain_request()), Ok(()));
}

#[test]
fn given_tools_when_the_model_takes_none_then_tools_not_supported() {
    let capabilities = caps_with(|spec| spec.tools = false);
    let with_tool = hi_request(one_tool(), OutputMode::Plain);
    assert_eq!(
        capabilities.check(&plain_settings(), &with_tool),
        Err(ClientError::ToolsNotSupported)
    );
    assert_eq!(
        capabilities.check(&plain_settings(), &plain_request()),
        Ok(())
    );
    let taking = caps_with(|spec| spec.tools = true);
    assert_eq!(taking.check(&plain_settings(), &with_tool), Ok(()));
}

#[test]
fn given_structured_output_when_the_model_cannot_give_it_then_structured_output_not_supported_with_or_without_tools()
 {
    let capabilities = caps_with(|spec| spec.structured_output = false);
    let alone = hi_request(vec![], shape());
    let with_tool = hi_request(one_tool(), shape());
    for request in [&alone, &with_tool] {
        assert_eq!(
            capabilities.check(&plain_settings(), request),
            Err(ClientError::StructuredOutputNotSupported)
        );
    }
    assert_eq!(
        capabilities.check(&plain_settings(), &plain_request()),
        Ok(())
    );
}

#[test]
fn given_structured_output_with_tools_when_the_model_cannot_combine_them_then_the_pair_is_refused()
{
    let capabilities = caps_with(|spec| spec.structured_output_with_tools = false);
    let combined = hi_request(one_tool(), shape());
    assert_eq!(
        capabilities.check(&plain_settings(), &combined),
        Err(ClientError::StructuredOutputWithToolsNotSupported)
    );
    let structured_alone = hi_request(vec![], shape());
    assert_eq!(
        capabilities.check(&plain_settings(), &structured_alone),
        Ok(())
    );
    let tools_plain = hi_request(one_tool(), OutputMode::Plain);
    assert_eq!(capabilities.check(&plain_settings(), &tools_plain), Ok(()));
}

#[test]
fn given_the_pair_flag_true_when_the_model_takes_the_pair_then_the_call_passes() {
    let combined = hi_request(one_tool(), shape());
    assert_eq!(
        Capabilities::everything().check(&plain_settings(), &combined),
        Ok(())
    );
}

#[test]
fn given_the_pair_flag_true_when_structured_output_is_false_then_step_five_refuses_not_step_six() {
    let capabilities = caps_with(|spec| {
        spec.structured_output = false;
        spec.structured_output_with_tools = true;
    });
    let combined = hi_request(one_tool(), shape());
    assert_eq!(
        capabilities.check(&plain_settings(), &combined),
        Err(ClientError::StructuredOutputNotSupported)
    );
}

#[test]
fn given_the_pair_flag_true_when_tools_is_false_then_step_four_refuses() {
    let capabilities = caps_with(|spec| {
        spec.tools = false;
        spec.structured_output_with_tools = true;
    });
    let combined = hi_request(one_tool(), shape());
    assert_eq!(
        capabilities.check(&plain_settings(), &combined),
        Err(ClientError::ToolsNotSupported)
    );
}

#[test]
fn given_a_history_with_an_unanswered_tool_call_in_an_open_turn_when_checked_then_the_check_does_not_look_at_the_history()
 {
    let fixture = Fixture::new();
    let reply = response(
        vec![ContentBlock::RuntimeToolCall(tool_call("search"))],
        ResponseEnding::tool_use(),
    );
    let thread = fixture.project(vec![
        human_turn(vec![text_block("then")]),
        ai_turn(turn_id(), fixture.agent, vec![reply.into()]),
    ]);
    let request = request_of(thread, vec![], OutputMode::Plain);
    assert_eq!(
        Capabilities::everything().check(&plain_settings(), &request),
        Ok(())
    );
    let refusing = caps_with(|spec| spec.caching = false);
    let required = settings(ReasoningLevel::Off, CacheRequirement::Required, 1000);
    assert_eq!(
        refusing.check(&required, &request),
        Err(ClientError::CacheNotSupported)
    );
}

#[test]
fn given_the_same_arguments_when_checked_twice_then_the_result_is_the_same() {
    let capabilities = Capabilities::nothing();
    let request = image_request(&[MediaType::Png]);
    let settings = level_settings(ReasoningLevel::High);
    let first = capabilities.check(&settings, &request);
    assert!(first.is_err());
    assert_eq!(first, capabilities.check(&settings, &request));
}
