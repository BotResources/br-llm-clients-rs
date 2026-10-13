use std::sync::Arc;

use br_llm_clients_core::{CallOutcome, ClientRequest, ClientSettings, StreamSink};

use crate::call_context::CallContext;
use crate::call_log::Served;
use crate::call_view::{MatchContext, ReplyContext};
use crate::claim::claim;
use crate::codes;
use crate::outcomes;
use crate::reply::Reply;
use crate::reply_source::Repr;
use crate::script::Claimed;
use crate::serve::serve;
use crate::shared::Shared;
use crate::strictness;

pub(crate) async fn drive(
    shared: &Arc<Shared>,
    settings: &ClientSettings,
    request: &ClientRequest,
    sink: &mut dyn StreamSink,
) -> CallOutcome {
    let begun = shared.begin(request, settings);
    let _guard = begun.guard;
    let mut call = CallContext {
        tracker: &begun.tracker,
        settings,
        request,
        sink,
    };
    let outcome = answer(&mut call, begun.rules_at_start).await;
    begun.tracker.finish(&outcome);
    outcome
}

async fn answer(call: &mut CallContext<'_>, rules_at_start: usize) -> CallOutcome {
    let capabilities = call.tracker.shared().capabilities();
    if let Err(error) = strictness::check(capabilities, call.settings, call.request) {
        call.tracker.serve(Served::Refusal(error.clone()));
        return outcomes::refused(error);
    }
    let Some(claimed) = claim(call.tracker, call.request, call.settings, rules_at_start) else {
        return outcomes::refused(codes::script_exhausted());
    };
    match reply_of(call, &claimed) {
        Ok(reply) => serve(call, reply).await,
        Err(outcome) => *outcome,
    }
}

fn reply_of(call: &CallContext<'_>, claimed: &Claimed) -> Result<Reply, Box<CallOutcome>> {
    match &claimed.source.repr {
        Repr::Value(reply) => Ok(Reply::clone(reply)),
        Repr::Function(function) => function(&ReplyContext::new(
            MatchContext::new(call.request, call.settings, call.tracker.sequence()),
            claimed.claimed_before,
        ))
        .map_err(|error| {
            Box::new(outcomes::refused(codes::invalid_reply(
                &origin(&claimed.served),
                &error,
            )))
        }),
    }
}

fn origin(served: &Served) -> String {
    match served {
        Served::Rule { position } => format!("the reply function of rule {position}"),
        Served::Entry { position } => format!("the reply function of script entry {position}"),
        Served::Fallback => "the reply function of the fallback".to_owned(),
        Served::NotYetClaimed | Served::Refusal(_) | Served::Exhausted | Served::Nothing => {
            "the reply function".to_owned()
        }
    }
}
