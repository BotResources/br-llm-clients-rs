use br_llm_clients_core::ClientError;
use br_llm_messages::{ModelResponse, Usage};

use crate::build_error::MockBuildError;
use crate::reply::Reply;
use crate::test_support::*;

fn with_usage(usage_value: Usage) -> ModelResponse {
    text_response_with_usage("hi", usage_value)
}

fn breaking_usage() -> Usage {
    Usage::of_input_tokens(5)
        .with_output_tokens(5)
        .with_cache_read_input_tokens(3)
        .with_cache_creation_input_tokens(3)
}

fn refused(usage_value: Usage) -> bool {
    matches!(
        Reply::success(with_usage(usage_value)).build(),
        Err(MockBuildError::UsageBreaksCounting)
    )
}

#[test]
fn given_a_usage_where_cache_tokens_exceed_input_when_built_then_usage_breaks_counting() {
    assert!(refused(breaking_usage()));
}

#[test]
fn given_thinking_tokens_above_output_when_built_then_usage_breaks_counting() {
    assert!(refused(
        Usage::of_input_tokens(5)
            .with_output_tokens(2)
            .with_thinking_tokens(3)
    ));
}

#[test]
fn given_a_cache_count_without_an_input_count_when_built_then_usage_breaks_counting() {
    assert!(refused(Usage::of_cache_read_input_tokens(0)));
    assert!(refused(Usage::of_cache_creation_input_tokens(0)));
    assert!(refused(
        Usage::of_output_tokens(4).with_cache_read_input_tokens(1)
    ));
}

#[test]
fn given_a_thinking_count_without_an_output_count_when_built_then_usage_breaks_counting() {
    assert!(refused(Usage::of_thinking_tokens(0)));
    assert!(refused(Usage::of_input_tokens(4).with_thinking_tokens(1)));
}

#[test]
fn given_a_usage_at_the_exact_bounds_when_built_then_accepted() {
    let usage_value = Usage::of_input_tokens(6)
        .with_output_tokens(3)
        .with_cache_read_input_tokens(3)
        .with_cache_creation_input_tokens(3)
        .with_thinking_tokens(3);
    assert!(Reply::success(with_usage(usage_value)).build().is_ok());
}

#[test]
fn given_an_absent_count_when_summed_then_it_counts_as_zero() {
    assert!(
        Reply::success(with_usage(
            Usage::of_input_tokens(2).with_cache_read_input_tokens(2)
        ))
        .build()
        .is_ok()
    );
    assert!(
        Reply::success(with_usage(
            Usage::of_output_tokens(2).with_thinking_tokens(2)
        ))
        .build()
        .is_ok()
    );
    assert!(
        Reply::success(with_usage(Usage::of_input_tokens(1)))
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
        Reply::cut(text_response("hi"), 1, error.clone())
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
    assert_eq!(
        Reply::failure(ClientError::Transport)
            .usage(
                Usage::of_input_tokens(1)
                    .with_output_tokens(1)
                    .with_cache_read_input_tokens(u64::MAX)
                    .with_cache_creation_input_tokens(u64::MAX)
            )
            .build()
            .unwrap_err(),
        MockBuildError::UsageBreaksCounting
    );
}

#[test]
fn given_cache_tokens_whose_sum_is_one_above_the_maximal_input_when_built_then_refused() {
    assert!(refused(
        Usage::of_input_tokens(u64::MAX)
            .with_output_tokens(1)
            .with_cache_read_input_tokens(u64::MAX)
            .with_cache_creation_input_tokens(1)
    ));
}

#[test]
fn given_cache_tokens_whose_sum_is_exactly_the_maximal_input_when_built_then_accepted() {
    assert!(
        Reply::success(with_usage(
            Usage::of_input_tokens(u64::MAX)
                .with_output_tokens(1)
                .with_cache_read_input_tokens(u64::MAX - 1)
                .with_cache_creation_input_tokens(1)
        ))
        .build()
        .is_ok()
    );
}

#[test]
fn given_thinking_tokens_one_above_the_maximal_output_when_built_then_refused() {
    assert!(refused(
        Usage::of_input_tokens(1)
            .with_output_tokens(u64::MAX - 1)
            .with_thinking_tokens(u64::MAX)
    ));
}

#[test]
fn given_thinking_tokens_equal_to_the_maximal_output_when_built_then_accepted() {
    assert!(
        Reply::success(with_usage(
            Usage::of_input_tokens(1)
                .with_output_tokens(u64::MAX)
                .with_thinking_tokens(u64::MAX)
        ))
        .build()
        .is_ok()
    );
}
