use br_llm_clients_core::CallOutcome;
use br_llm_messages::StreamEvent;

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
    stream(call, plan.events, plan.gates).await;
    match plan.ending {
        Ending::Outcome(outcome) => *outcome,
        Ending::Hang => hang(call.tracker).await,
    }
}

async fn stream(
    call: &mut CallContext<'_>,
    events: Vec<StreamEvent>,
    mut gates: Vec<(usize, Gate)>,
) {
    gates.sort_by_key(|(position, _)| *position);
    let mut next_gate = 0;
    let mut position = 0;
    let mut events = events.into_iter();
    loop {
        while next_gate < gates.len() && gates[next_gate].0 == position {
            gates[next_gate].1.pass(call.tracker.clone()).await;
            next_gate += 1;
        }
        let Some(event) = events.next() else {
            break;
        };
        call.sink.send(event).await;
        position += 1;
    }
}

async fn hang(tracker: &CallTracker) -> CallOutcome {
    tracker.announce(CallState::Hanging);
    std::future::pending().await
}
