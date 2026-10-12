use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::derive_events::derive_events;
use crate::reply::{Reply, Shape};
use crate::usage_rules;

pub(crate) fn check(reply: &Reply) -> Result<(), MockBuildError> {
    check_usages(reply)?;
    reply.chunking.check()?;
    check_step_settings(reply)?;
    check_report_times(reply)?;
    let before_finish = match reply.step() {
        Some(step) => Some(derive_events(step, &reply.chunking)?.before_finish()),
        None => None,
    };
    check_event_count(reply, before_finish)?;
    let sendable = sendable_events(reply, before_finish);
    check_gates(reply, sendable)?;
    check_first_event(reply, sendable)
}

fn check_usages(reply: &Reply) -> Result<(), MockBuildError> {
    if let Some(usage) = reply.step().and_then(|step| step.usage()) {
        usage_rules::check(usage)?;
    }
    if let Some(usage) = &reply.report.usage {
        usage_rules::check(usage)?;
    }
    Ok(())
}

fn check_step_settings(reply: &Reply) -> Result<(), MockBuildError> {
    let has_setting = reply.chunking != Chunking::none() || reply.renaming.is_some();
    if has_setting && reply.step().is_none() {
        return Err(MockBuildError::StepSettingWithoutStep);
    }
    Ok(())
}

fn check_report_times(reply: &Reply) -> Result<(), MockBuildError> {
    if let (Some(first), Some(total)) = (
        reply.report.time_to_first_event,
        reply.report.total_duration,
    ) && first > total
    {
        return Err(MockBuildError::FirstEventAfterEnd);
    }
    Ok(())
}

fn check_event_count(reply: &Reply, before_finish: Option<usize>) -> Result<(), MockBuildError> {
    let (Shape::Cut { after, .. } | Shape::Hang { after, .. }) = &reply.shape else {
        return Ok(());
    };
    let available = before_finish.unwrap_or(0);
    if *after > available {
        return Err(MockBuildError::EventCountOutOfRange {
            requested: *after,
            available,
        });
    }
    Ok(())
}

fn sendable_events(reply: &Reply, before_finish: Option<usize>) -> usize {
    match &reply.shape {
        Shape::Failure(_) => 0,
        Shape::Cut { after, .. } | Shape::Hang { after, .. } => *after,
        Shape::Success(_) => before_finish.unwrap_or(0),
        Shape::Broken { events, .. } => events.len(),
    }
}

fn check_gates(reply: &Reply, sendable: usize) -> Result<(), MockBuildError> {
    for (position, _) in &reply.gates {
        if *position > sendable {
            return Err(MockBuildError::GatePositionOutOfRange {
                position: *position,
                available: sendable,
            });
        }
    }
    Ok(())
}

fn check_first_event(reply: &Reply, sendable: usize) -> Result<(), MockBuildError> {
    let sends_nothing = !matches!(reply.shape, Shape::Success(_)) && sendable == 0;
    if sends_nothing && reply.report.time_to_first_event.is_some() {
        return Err(MockBuildError::FirstEventWithoutEvent);
    }
    Ok(())
}
