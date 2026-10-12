use br_llm_clients_core::{ClientRequest, ClientSettings};

use crate::call_log::Served;
use crate::script::Claimed;
use crate::tracker::CallTracker;

struct PanicMark<'a> {
    tracker: &'a CallTracker,
    armed: bool,
}

impl PanicMark<'_> {
    fn defuse(mut self) {
        self.armed = false;
    }
}

impl Drop for PanicMark<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.tracker.serve(Served::Nothing);
        }
    }
}

pub(crate) fn claim(
    tracker: &CallTracker,
    request: &ClientRequest,
    settings: &ClientSettings,
    rules_at_start: usize,
) -> Option<Claimed> {
    let shared = tracker.shared();
    let sequence = tracker.sequence();
    for position in 0..rules_at_start {
        let Some(matcher) = shared.with(|state| state.script.candidate(position)) else {
            continue;
        };
        let mark = PanicMark {
            tracker,
            armed: true,
        };
        let matched = matcher(request, settings, sequence);
        mark.defuse();
        if !matched {
            continue;
        }
        let claimed = shared.with(|state| {
            let claimed = state.script.claim_rule(position)?;
            state.log.set_served(sequence, claimed.served.clone());
            Some(claimed)
        });
        if claimed.is_some() {
            return claimed;
        }
    }
    shared.with(|state| {
        let claimed = state.script.claim_remaining();
        let served = claimed
            .as_ref()
            .map_or(Served::Exhausted, |claimed| claimed.served.clone());
        state.log.set_served(sequence, served);
        claimed
    })
}
