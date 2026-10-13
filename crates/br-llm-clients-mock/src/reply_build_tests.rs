use std::time::Duration;

use br_llm_clients_core::ClientError;

use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::derive_events::derive_events;
use crate::gate::Gate;
use crate::reply::Reply;
use crate::test_support::*;

fn ok_step() -> br_llm_messages::Step {
    text_step("hello")
}

fn event_count() -> usize {
    derive_events(&ok_step(), &Chunking::none())
        .unwrap()
        .before_finish()
}

#[test]
fn given_a_count_above_the_events_before_finish_when_built_then_event_count_out_of_range() {
    let count = event_count();
    for reply in [
        Reply::cut(ok_step(), count + 1, ClientError::Transport).build(),
        Reply::hang(ok_step(), count + 1).build(),
    ] {
        assert_eq!(
            reply.unwrap_err(),
            MockBuildError::EventCountOutOfRange {
                requested: count + 1,
                available: count
            }
        );
    }
}

#[test]
fn given_a_count_of_zero_or_the_full_count_when_built_then_accepted() {
    let count = event_count();
    for after in [0, count] {
        assert!(
            Reply::cut(ok_step(), after, ClientError::Transport)
                .build()
                .is_ok()
        );
        assert!(Reply::hang(ok_step(), after).build().is_ok());
    }
}

#[test]
fn given_a_chunking_when_the_count_is_checked_then_the_count_follows_the_chunking() {
    let chunking = Chunking::none().text(1);
    let derived = derive_events(&ok_step(), &chunking)
        .unwrap()
        .before_finish();
    assert!(
        Reply::cut(ok_step(), derived, ClientError::Transport)
            .chunking(chunking)
            .build()
            .is_ok()
    );
    assert!(
        Reply::cut(ok_step(), derived + 1, ClientError::Transport)
            .chunking(chunking)
            .build()
            .is_err()
    );
}

#[test]
fn given_a_chunk_size_of_zero_when_built_then_chunk_size_zero() {
    assert_eq!(
        Reply::success(ok_step())
            .chunking(Chunking::none().text(0))
            .build()
            .unwrap_err(),
        MockBuildError::ChunkSizeZero
    );
}

#[test]
fn given_a_chunking_or_a_renaming_on_a_reply_without_step_when_built_then_step_setting_without_step()
 {
    let chunked = Chunking::none().text(2);
    assert_eq!(
        Reply::failure(ClientError::Transport)
            .chunking(chunked)
            .build()
            .unwrap_err(),
        MockBuildError::StepSettingWithoutStep
    );
    assert_eq!(
        Reply::failure(ClientError::Transport)
            .rename_tool_call_ids("c")
            .build()
            .unwrap_err(),
        MockBuildError::StepSettingWithoutStep
    );
    assert_eq!(
        Reply::broken_stream(vec![], ClientError::Transport)
            .chunking(chunked)
            .build()
            .unwrap_err(),
        MockBuildError::StepSettingWithoutStep
    );
    assert_eq!(
        Reply::broken_stream(vec![], ClientError::Transport)
            .rename_tool_call_ids("c")
            .build()
            .unwrap_err(),
        MockBuildError::StepSettingWithoutStep
    );
    assert!(
        Reply::cut(ok_step(), 1, ClientError::Transport)
            .chunking(chunked)
            .rename_tool_call_ids("c")
            .build()
            .is_ok()
    );
    assert!(
        Reply::hang(ok_step(), 1)
            .chunking(chunked)
            .rename_tool_call_ids("c")
            .build()
            .is_ok()
    );
}

#[test]
fn given_the_first_event_after_the_total_when_built_then_first_event_after_end() {
    let reply = Reply::success(ok_step())
        .time_to_first_event(Duration::from_millis(5))
        .total_duration(Duration::from_millis(4))
        .build();
    assert_eq!(reply.unwrap_err(), MockBuildError::FirstEventAfterEnd);
}

#[test]
fn given_equal_or_ordered_times_when_built_then_accepted() {
    for (first, total) in [(5, 5), (0, 9)] {
        assert!(
            Reply::success(ok_step())
                .time_to_first_event(Duration::from_millis(first))
                .total_duration(Duration::from_millis(total))
                .build()
                .is_ok()
        );
    }
}

#[test]
fn given_only_one_of_the_times_when_built_then_the_check_does_not_apply() {
    assert!(
        Reply::success(ok_step())
            .time_to_first_event(Duration::from_secs(99))
            .build()
            .is_ok()
    );
    assert!(
        Reply::success(ok_step())
            .total_duration(Duration::from_millis(1))
            .build()
            .is_ok()
    );
}

#[test]
fn given_a_first_event_time_on_a_reply_that_sends_no_event_when_built_then_first_event_without_event()
 {
    let first = Duration::from_millis(1);
    assert_eq!(
        Reply::failure(ClientError::Transport)
            .time_to_first_event(first)
            .build()
            .unwrap_err(),
        MockBuildError::FirstEventWithoutEvent
    );
    assert_eq!(
        Reply::cut(ok_step(), 0, ClientError::Transport)
            .time_to_first_event(first)
            .build()
            .unwrap_err(),
        MockBuildError::FirstEventWithoutEvent
    );
    assert_eq!(
        Reply::broken_stream(vec![], ClientError::Transport)
            .time_to_first_event(first)
            .build()
            .unwrap_err(),
        MockBuildError::FirstEventWithoutEvent
    );
    assert!(
        Reply::cut(ok_step(), 1, ClientError::Transport)
            .time_to_first_event(first)
            .build()
            .is_ok()
    );
    assert!(
        Reply::broken_stream(vec![finish_event()], ClientError::Transport)
            .time_to_first_event(first)
            .build()
            .is_ok()
    );
}

#[test]
fn given_the_same_settings_in_any_order_when_built_then_the_outcome_is_the_same() {
    let gate = Gate::new();
    let first = Reply::success(ok_step())
        .gate_at(1, &gate)
        .chunking(Chunking::none().text(0))
        .build();
    let second = Reply::success(ok_step())
        .chunking(Chunking::none().text(0))
        .gate_at(1, &gate)
        .build();
    assert_eq!(first.unwrap_err(), second.unwrap_err());
}

#[test]
fn given_a_reply_that_breaks_a_rule_when_built_then_it_returns_an_error_and_does_not_panic() {
    let result =
        std::panic::catch_unwind(|| Reply::cut(text_step("x"), 99, ClientError::Transport).build());
    assert!(result.unwrap().is_err());
}

#[test]
fn given_a_valid_reply_of_each_kind_when_built_then_accepted() {
    let step_value = ok_step();
    assert!(Reply::success(step_value.clone()).build().is_ok());
    assert!(Reply::failure(ClientError::QuotaExceeded).build().is_ok());
    assert!(
        Reply::cut(step_value.clone(), 1, ClientError::StreamTruncated)
            .build()
            .is_ok()
    );
    assert!(Reply::hang(step_value, 1).build().is_ok());
    assert!(
        Reply::broken_stream(vec![finish_event(), finish_event()], ClientError::Transport)
            .build()
            .is_ok()
    );
}
