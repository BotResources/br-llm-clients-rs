use br_llm_clients_core::CallOutcome;
use br_llm_messages::ResponseDelta;

use crate::call_context::CallContext;
use crate::call_log::CallState;
use crate::gate::Gate;
use crate::plan::{Ending, plan};
use crate::reply::Reply;
use crate::tracker::CallTracker;

pub(crate) async fn serve(call: &mut CallContext<'_>, reply: Reply) -> CallOutcome {
    let plan = match plan(call, reply) {
        Ok(plan) => plan,
        Err(outcome) => return *outcome,
    };
    stream(call, plan.deltas, plan.gates).await;
    match plan.ending {
        Ending::Outcome(outcome) => *outcome,
        Ending::Hang => hang(call.tracker).await,
    }
}

async fn stream(
    call: &mut CallContext<'_>,
    deltas: Vec<ResponseDelta>,
    mut gates: Vec<(usize, Gate)>,
) {
    gates.sort_by_key(|(position, _)| *position);
    let mut gates = gates.into_iter().peekable();
    let mut position = 0;
    let mut deltas = deltas.into_iter();
    loop {
        while let Some((_, gate)) = gates.next_if(|(gate_position, _)| *gate_position == position) {
            gate.pass(call.tracker.clone()).await;
        }
        let Some(delta) = deltas.next() else {
            break;
        };
        call.sink.send(delta).await;
        position = position.saturating_add(1);
    }
}

async fn hang(tracker: &CallTracker) -> CallOutcome {
    tracker.announce(CallState::Hanging);
    std::future::pending().await
}
