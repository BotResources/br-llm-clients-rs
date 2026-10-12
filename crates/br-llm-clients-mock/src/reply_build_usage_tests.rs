use br_llm_clients_core::ClientError;
use br_llm_messages::Usage;

use crate::build_error::MockBuildError;
use crate::reply::Reply;
use crate::test_support::*;

fn ok_step() -> br_llm_messages::Step {
    text_step("hello")
}

fn breaking_usage() -> Usage {
    Usage {
        input_tokens: 5,
        output_tokens: 5,
        cache_read_tokens: Some(3),
        cache_write_tokens: Some(3),
        thinking_tokens: None,
    }
}

#[test]
fn given_a_step_usage_where_cache_tokens_exceed_input_when_built_then_usage_breaks_counting() {
    let step_value = step_with_usage("hi", breaking_usage());
    assert_eq!(
        Reply::success(step_value).build().unwrap_err(),
        MockBuildError::UsageBreaksCounting
    );
}

#[test]
fn given_thinking_tokens_above_output_when_built_then_usage_breaks_counting() {
    let usage_value = Usage {
        input_tokens: 5,
        output_tokens: 2,
        thinking_tokens: Some(3),
        ..Usage::default()
    };
    assert_eq!(
        Reply::success(step_with_usage("hi", usage_value))
            .build()
            .unwrap_err(),
        MockBuildError::UsageBreaksCounting
    );
}

#[test]
fn given_a_usage_at_the_exact_bounds_when_built_then_accepted() {
    let usage_value = Usage {
        input_tokens: 6,
        output_tokens: 3,
        cache_read_tokens: Some(3),
        cache_write_tokens: Some(3),
        thinking_tokens: Some(3),
    };
    assert!(
        Reply::success(step_with_usage("hi", usage_value))
            .build()
            .is_ok()
    );
}

#[test]
fn given_a_named_usage_that_breaks_counting_when_built_then_every_reply_that_names_one_refuses() {
    let error = ClientError::Transport;
    assert_eq!(
        Reply::failure(error.clone())
            .usage(breaking_usage())
            .build()
            .unwrap_err(),
        MockBuildError::UsageBreaksCounting
    );
    assert_eq!(
        Reply::cut(ok_step(), 1, error.clone())
            .usage(breaking_usage())
            .build()
            .unwrap_err(),
        MockBuildError::UsageBreaksCounting
    );
    assert_eq!(
        Reply::broken_stream(vec![], error)
            .usage(breaking_usage())
            .build()
            .unwrap_err(),
        MockBuildError::UsageBreaksCounting
    );
}

#[test]
fn given_a_usage_that_overflows_when_summed_then_it_is_refused_without_panic() {
    let usage_value = Usage {
        input_tokens: 1,
        output_tokens: 1,
        cache_read_tokens: Some(u64::MAX),
        cache_write_tokens: Some(u64::MAX),
        thinking_tokens: None,
    };
    assert_eq!(
        Reply::failure(ClientError::Transport)
            .usage(usage_value)
            .build()
            .unwrap_err(),
        MockBuildError::UsageBreaksCounting
    );
}
