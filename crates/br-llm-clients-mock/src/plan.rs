use br_llm_clients_core::CallOutcome;
use br_llm_messages::{ModelResponse, ResponseDelta};

use crate::call_context::CallContext;
use crate::chunking::Chunking;
use crate::codes;
use crate::derive_deltas::derive_response_deltas;
use crate::gate::Gate;
use crate::outcomes;
use crate::renew::renew;
use crate::reply::{Reply, Shape};
use crate::report_values::ReportValues;
use crate::response_check;

pub(crate) enum Ending {
    Outcome(Box<CallOutcome>),
    Hang,
}

pub(crate) struct Plan {
    pub(crate) deltas: Vec<ResponseDelta>,
    pub(crate) gates: Vec<(usize, Gate)>,
    pub(crate) ending: Ending,
}

fn internal(cause: &impl std::fmt::Display) -> Box<CallOutcome> {
    Box::new(outcomes::refused(outcomes::inconsistent(cause)))
}

fn collision() -> Box<CallOutcome> {
    Box::new(outcomes::refused(codes::identity_collision()))
}

fn renewed(
    call: &CallContext<'_>,
    response: &ModelResponse,
) -> Result<ModelResponse, Box<CallOutcome>> {
    renew(
        call.tracker.shared().id_source(),
        call.request.reply_entry(),
        response,
    )
    .map_err(|_| collision())
}

fn derived(
    call: &CallContext<'_>,
    response: &ModelResponse,
    chunking: &Chunking,
) -> Result<Vec<ResponseDelta>, Box<CallOutcome>> {
    derive_response_deltas(call.request.reply_turn(), response, chunking)
        .map(crate::derive_deltas::DerivedDeltas::into_deltas)
        .map_err(|error| internal(&error))
}

fn of_request(call: &CallContext<'_>, deltas: Vec<ResponseDelta>) -> Vec<ResponseDelta> {
    deltas
        .into_iter()
        .map(|delta| {
            ResponseDelta::new(
                call.request.reply_turn(),
                call.request.reply_entry(),
                delta.op().clone(),
            )
        })
        .collect()
}

pub(crate) fn plan(call: &CallContext<'_>, reply: Reply) -> Result<Plan, Box<CallOutcome>> {
    let Reply {
        shape,
        report,
        gates,
        chunking,
        request_identities,
    } = reply;
    let (deltas, ending) = match shape {
        Shape::Failure(error) => (Vec::new(), failure(&report, error)),
        Shape::Broken { deltas, error } => {
            let deltas = if request_identities {
                of_request(call, deltas)
            } else {
                deltas
            };
            (deltas, failure(&report, error))
        }
        Shape::Cut {
            response,
            after,
            error,
        } => {
            let response = renewed(call, &response)?;
            let mut deltas = derived(call, &response, &chunking)?;
            deltas.truncate(after);
            (deltas, failure(&report, error))
        }
        Shape::Hang { response, after } => {
            let response = renewed(call, &response)?;
            let mut deltas = derived(call, &response, &chunking)?;
            deltas.truncate(after);
            (deltas, Ending::Hang)
        }
        Shape::Success(response) => {
            let response = renewed(call, &response)?;
            success_plan(call, response, &report, &chunking)?
        }
    };
    Ok(Plan {
        deltas,
        gates,
        ending,
    })
}

fn failure(report: &ReportValues, error: br_llm_clients_core::ClientError) -> Ending {
    Ending::Outcome(Box::new(outcomes::failed(report.named_report(), error)))
}

fn success_plan(
    call: &CallContext<'_>,
    response: ModelResponse,
    report: &ReportValues,
    chunking: &Chunking,
) -> Result<(Vec<ResponseDelta>, Ending), Box<CallOutcome>> {
    let mut deltas = derived(call, &response, chunking)?;
    let completed = deltas.pop();
    let response_report = report.report(response.usage().cloned(), response.model().cloned());
    let capabilities = call.tracker.shared().capabilities();
    match response_check::check(capabilities, call.request, &response) {
        Err(error) => Ok((
            deltas,
            Ending::Outcome(Box::new(outcomes::failed(response_report, error))),
        )),
        Ok(()) => {
            deltas.extend(completed);
            Ok((
                deltas,
                Ending::Outcome(Box::new(outcomes::succeeded(response_report, response))),
            ))
        }
    }
}
