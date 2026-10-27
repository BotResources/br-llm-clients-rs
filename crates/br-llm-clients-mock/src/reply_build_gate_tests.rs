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
fn given_a_gate_position_above_what_the_reply_sends_when_built_then_gate_position_out_of_range() {
    let gate = Gate::new();
    let count = delta_count();
    let cases = [
        (
            Reply::failure(ClientError::Transport)
                .gate_at(1, &gate)
                .build(),
            0,
        ),
        (
            Reply::cut(ok_response(), 2, ClientError::Transport)
                .gate_at(3, &gate)
                .build(),
            2,
        ),
        (Reply::hang(ok_response(), 2).gate_at(3, &gate).build(), 2),
        (
            Reply::success(ok_response())
                .gate_at(count + 1, &gate)
                .build(),
            count,
        ),
        (
            Reply::broken_stream(vec![started_delta()], ClientError::Transport)
                .gate_at(2, &gate)
                .build(),
            1,
        ),
    ];
    for (reply, available) in cases {
        assert!(matches!(
            reply.unwrap_err(),
            MockBuildError::GatePositionOutOfRange { available: found, .. } if found == available
        ));
    }
}

#[test]
fn given_gate_positions_at_the_limits_when_built_then_accepted() {
    let gate = Gate::new();
    let count = delta_count();
    assert!(
        Reply::failure(ClientError::Transport)
            .gate_at(0, &gate)
            .build()
            .is_ok()
    );
    assert!(
        Reply::cut(ok_response(), 2, ClientError::Transport)
            .gate_at(2, &gate)
            .build()
            .is_ok()
    );
    assert!(
        Reply::hang(ok_response(), 2)
            .gate_at(0, &gate)
            .gate_at(2, &gate)
            .build()
            .is_ok()
    );
    assert!(
        Reply::success(ok_response())
            .gate_at(count, &gate)
            .gate_at(0, &gate)
            .build()
            .is_ok()
    );
    assert!(
        Reply::broken_stream(vec![started_delta()], ClientError::Transport)
            .gate_at(1, &gate)
            .build()
            .is_ok()
    );
}

#[test]
fn given_a_chunking_when_a_gate_is_checked_then_the_available_count_follows_the_chunking() {
    let gate = Gate::new();
    let chunking = Chunking::none().text(1);
    let derived = count_before_completed(&ok_response(), &chunking).unwrap();
    assert!(
        Reply::success(ok_response())
            .chunking(chunking)
            .gate_at(derived, &gate)
            .build()
            .is_ok()
    );
    assert!(
        Reply::success(ok_response())
            .gate_at(derived, &gate)
            .build()
            .is_err()
    );
}
