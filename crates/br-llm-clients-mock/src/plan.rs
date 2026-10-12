use br_llm_clients_core::{CallOutcome, ClientError};
use br_llm_messages::{MessageError, Step, StreamEvent};

use crate::call_context::CallContext;
use crate::chunking::Chunking;
use crate::codes;
use crate::derive_events::derive_events;
use crate::gate::Gate;
use crate::outcomes;
use crate::reply::{Reply, Shape};
use crate::report_values::ReportValues;
use crate::step_check;
use crate::strictness::classify_mode;
use crate::tool_call_id::rename_tool_calls;

pub(crate) enum Ending {
    Outcome(Box<CallOutcome>),
    Hang,
}

pub(crate) struct Plan {
    pub(crate) events: Vec<StreamEvent>,
    pub(crate) gates: Vec<(usize, Gate)>,
    pub(crate) ending: Ending,
}

fn internal(cause: &impl std::fmt::Display) -> Box<CallOutcome> {
    Box::new(outcomes::refused(codes::invalid_reply("the reply", cause)))
}

fn renamed(
    call: &CallContext<'_>,
    step: Step,
    renaming: Option<&str>,
) -> Result<Step, MessageError> {
    match renaming {
        Some(prefix) => rename_tool_calls(&step, prefix, call.tracker.sequence()),
        None => Ok(step),
    }
}

fn with_model(
    call: &CallContext<'_>,
    step: Step,
    report: &ReportValues,
) -> Result<Step, MessageError> {
    if step.model().is_some() {
        return Ok(step);
    }
    let model = report
        .model
        .clone()
        .unwrap_or_else(|| call.settings.model().clone());
    Step::new(
        step.content().to_vec(),
        step.stop_reason().clone(),
        step.usage().cloned(),
        Some(model),
    )
}

fn truncated(
    step: &Step,
    chunking: &Chunking,
    after: usize,
) -> Result<Vec<StreamEvent>, Box<CallOutcome>> {
    let mut events = derive_events(step, chunking)
        .map_err(|error| internal(&error))?
        .into_events();
    events.truncate(after);
    Ok(events)
}

pub(crate) fn plan(call: &CallContext<'_>, reply: Reply) -> Result<Plan, Box<CallOutcome>> {
    let Reply {
        shape,
        report,
        gates,
        chunking,
        renaming,
    } = reply;
    let named_usage = report.usage.clone();
    let (events, ending) = match shape {
        Shape::Failure(error) => (
            Vec::new(),
            Ending::Outcome(Box::new(outcomes::failed(
                report.report(named_usage),
                error,
            ))),
        ),
        Shape::Broken { events, error } => (
            events,
            Ending::Outcome(Box::new(outcomes::failed(
                report.report(named_usage),
                error,
            ))),
        ),
        Shape::Cut { step, after, error } => {
            let step =
                renamed(call, step, renaming.as_deref()).map_err(|error| internal(&error))?;
            (
                truncated(&step, &chunking, after)?,
                Ending::Outcome(Box::new(outcomes::failed(
                    report.report(named_usage),
                    error,
                ))),
            )
        }
        Shape::Hang { step, after } => {
            let step =
                renamed(call, step, renaming.as_deref()).map_err(|error| internal(&error))?;
            (truncated(&step, &chunking, after)?, Ending::Hang)
        }
        Shape::Success(step) => {
            let step = renamed(call, step, renaming.as_deref())
                .and_then(|step| with_model(call, step, &report))
                .map_err(|error| internal(&error))?;
            success_plan(call, step, &report, &chunking)?
        }
    };
    Ok(Plan {
        events,
        gates,
        ending,
    })
}

fn success_plan(
    call: &CallContext<'_>,
    step: Step,
    report: &ReportValues,
    chunking: &Chunking,
) -> Result<(Vec<StreamEvent>, Ending), Box<CallOutcome>> {
    let mut events = derive_events(&step, chunking)
        .map_err(|error| internal(&error))?
        .into_events();
    let finish = events.pop();
    let step_report = report.report(step.usage().cloned());
    let verdict = check_against_request(call, &step);
    match verdict {
        Err(error) => Ok((
            events,
            Ending::Outcome(Box::new(outcomes::failed(step_report, error))),
        )),
        Ok(()) => {
            events.extend(finish);
            Ok((
                events,
                Ending::Outcome(Box::new(outcomes::succeeded(step_report, step))),
            ))
        }
    }
}

fn check_against_request(call: &CallContext<'_>, step: &Step) -> Result<(), ClientError> {
    step_check::check(
        call.tracker.shared().capabilities(),
        call.request,
        classify_mode(call.request.output()),
        step,
    )
}
