use std::time::Duration;

use br_llm_clients_core::ClientError;

use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::gate::Gate;
use crate::reply::Reply;
use crate::reply_check::count_before_completed;
use crate::test_support::*;

fn ok_response() -> br_llm_messages::ModelResponse {
    text_response("hello")
}

fn delta_count() -> usize {
    count_before_completed(&ok_response(), &Chunking::none()).unwrap()
}

#[test]
fn given_a_count_above_the_deltas_before_completed_when_built_then_delta_count_out_of_range() {
    let count = delta_count();
    for reply in [
        Reply::cut(ok_response(), count + 1, ClientError::Transport).build(),
        Reply::hang(ok_response(), count + 1).build(),
    ] {
        assert_eq!(
            reply.unwrap_err(),
            MockBuildError::DeltaCountOutOfRange {
                requested: count + 1,
                available: count
            }
        );
    }
}

#[test]
fn given_a_count_of_zero_or_the_full_count_when_built_then_accepted() {
    let count = delta_count();
    for after in [0, count] {
        assert!(
            Reply::cut(ok_response(), after, ClientError::Transport)
                .build()
                .is_ok()
        );
        assert!(Reply::hang(ok_response(), after).build().is_ok());
    }
}

#[test]
fn given_a_chunking_when_the_count_is_checked_then_the_count_follows_the_chunking() {
    let chunking = Chunking::none().text(1);
    let derived = count_before_completed(&ok_response(), &chunking).unwrap();
    assert!(derived > delta_count());
    assert!(
        Reply::cut(ok_response(), derived, ClientError::Transport)
            .chunking(chunking)
            .build()
            .is_ok()
    );
    assert_eq!(
        Reply::cut(ok_response(), derived + 1, ClientError::Transport)
            .chunking(chunking)
            .build()
            .unwrap_err(),
        MockBuildError::DeltaCountOutOfRange {
            requested: derived + 1,
            available: derived
        }
    );
}

#[test]
fn given_a_chunk_size_of_zero_when_built_then_chunk_size_zero() {
    for chunking in [
        Chunking::none().text(0),
        Chunking::none().thinking(0),
        Chunking::none().tool_arguments(0),
        Chunking::none().structured(0),
    ] {
        assert_eq!(
            Reply::success(ok_response())
                .chunking(chunking)
                .build()
                .unwrap_err(),
            MockBuildError::ChunkSizeZero
        );
    }
}

#[test]
fn given_a_chunking_on_a_reply_without_response_when_built_then_chunking_without_response() {
    let chunked = Chunking::none().text(2);
    assert_eq!(
        Reply::failure(ClientError::Transport)
            .chunking(chunked)
            .build()
            .unwrap_err(),
        MockBuildError::ChunkingWithoutResponse
    );
    assert_eq!(
        Reply::broken_stream(vec![], ClientError::Transport)
            .chunking(chunked)
            .build()
            .unwrap_err(),
        MockBuildError::ChunkingWithoutResponse
    );
    assert!(
        Reply::cut(ok_response(), 1, ClientError::Transport)
            .chunking(chunked)
            .build()
            .is_ok()
    );
    assert!(
        Reply::hang(ok_response(), 1)
            .chunking(chunked)
            .build()
            .is_ok()
    );
}

#[test]
fn given_the_first_delta_after_the_total_when_built_then_first_event_after_end() {
    let reply = Reply::success(ok_response())
        .time_to_first_event(Duration::from_millis(5))
        .total_duration(Duration::from_millis(4))
        .build();
    assert_eq!(reply.unwrap_err(), MockBuildError::FirstEventAfterEnd);
}

#[test]
fn given_equal_or_ordered_times_when_built_then_accepted() {
    for (first, total) in [(5, 5), (0, 9)] {
        assert!(
            Reply::success(ok_response())
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
        Reply::success(ok_response())
            .time_to_first_event(Duration::from_secs(99))
            .build()
            .is_ok()
    );
    assert!(
        Reply::success(ok_response())
            .total_duration(Duration::from_millis(1))
            .build()
            .is_ok()
    );
}

#[test]
fn given_a_first_delta_time_on_a_reply_that_sends_no_delta_when_built_then_first_event_without_event()
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
        Reply::cut(ok_response(), 0, ClientError::Transport)
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
        Reply::cut(ok_response(), 1, ClientError::Transport)
            .time_to_first_event(first)
            .build()
            .is_ok()
    );
    assert!(
        Reply::broken_stream(vec![started_delta()], ClientError::Transport)
            .time_to_first_event(first)
            .build()
            .is_ok()
    );
}

#[test]
fn given_the_same_settings_in_any_order_when_built_then_the_outcome_is_the_same() {
    let gate = Gate::new();
    let first = Reply::success(ok_response())
        .gate_at(1, &gate)
        .chunking(Chunking::none().text(0))
        .build();
    let second = Reply::success(ok_response())
        .chunking(Chunking::none().text(0))
        .gate_at(1, &gate)
        .build();
    assert_eq!(first.unwrap_err(), second.unwrap_err());
}

#[test]
fn given_a_valid_reply_of_each_kind_when_built_then_accepted() {
    assert!(Reply::success(ok_response()).build().is_ok());
    assert!(Reply::failure(ClientError::QuotaExceeded).build().is_ok());
    assert!(
        Reply::cut(ok_response(), 1, ClientError::StreamTruncated)
            .build()
            .is_ok()
    );
    assert!(Reply::hang(ok_response(), 1).build().is_ok());
    assert!(
        Reply::broken_stream(
            vec![started_delta(), started_delta()],
            ClientError::Transport
        )
        .build()
        .is_ok()
    );
}

#[test]
fn given_a_reply_that_breaks_a_rule_when_built_then_it_returns_an_error_and_does_not_panic() {
    let result = std::panic::catch_unwind(|| {
        Reply::cut(text_response("x"), 99, ClientError::Transport).build()
    });
    assert!(result.unwrap().is_err());
}
