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
fn given_a_gate_position_above_what_the_reply_sends_when_built_then_gate_position_out_of_range() {
    let gate = Gate::new();
    let count = event_count();
    let cases = [
        (
            Reply::failure(ClientError::Transport)
                .gate_at(1, &gate)
                .build(),
            0,
        ),
        (
            Reply::cut(ok_step(), 2, ClientError::Transport)
                .gate_at(3, &gate)
                .build(),
            2,
        ),
        (Reply::hang(ok_step(), 2).gate_at(3, &gate).build(), 2),
        (
            Reply::success(ok_step()).gate_at(count + 1, &gate).build(),
            count,
        ),
        (
            Reply::broken_stream(vec![finish_event()], ClientError::Transport)
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
    let count = event_count();
    assert!(
        Reply::failure(ClientError::Transport)
            .gate_at(0, &gate)
            .build()
            .is_ok()
    );
    assert!(
        Reply::cut(ok_step(), 2, ClientError::Transport)
            .gate_at(2, &gate)
            .build()
            .is_ok()
    );
    assert!(
        Reply::hang(ok_step(), 2)
            .gate_at(0, &gate)
            .gate_at(2, &gate)
            .build()
            .is_ok()
    );
    assert!(
        Reply::success(ok_step())
            .gate_at(count, &gate)
            .gate_at(0, &gate)
            .build()
            .is_ok()
    );
    assert!(
        Reply::broken_stream(vec![finish_event()], ClientError::Transport)
            .gate_at(1, &gate)
            .build()
            .is_ok()
    );
}
