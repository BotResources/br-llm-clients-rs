use br_llm_messages::{ModelResponse, TurnId};

use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::derive_deltas::derive_response_deltas;
use crate::reply::{Reply, Shape};
use crate::sequential_id_source::identity_uuid;
use crate::usage_rules;

pub(crate) fn check(reply: &Reply) -> Result<(), MockBuildError> {
    check_usages(reply)?;
    reply.chunking.check()?;
    check_chunking_target(reply)?;
    check_report_times(reply)?;
    let before_completed = match reply.response() {
        Some(response) => Some(count_before_completed(response, &reply.chunking)?),
        None => None,
    };
    check_delta_count(reply, before_completed)?;
    let sendable = sendable_deltas(reply, before_completed);
    check_gates(reply, sendable)?;
    check_first_delta(reply, sendable)
}

pub(crate) fn count_before_completed(
    response: &ModelResponse,
    chunking: &Chunking,
) -> Result<usize, MockBuildError> {
    let turn =
        TurnId::from_uuid(identity_uuid(0)).map_err(|error| MockBuildError::DerivationFailed {
            reason: error.to_string(),
        })?;
    Ok(derive_response_deltas(turn, response, chunking)?.before_completed())
}

fn check_usages(reply: &Reply) -> Result<(), MockBuildError> {
    if let Some(usage) = reply.response().and_then(ModelResponse::usage) {
        usage_rules::check(usage)?;
    }
    if let Some(usage) = &reply.report.usage {
        usage_rules::check(usage)?;
    }
    Ok(())
}

fn check_chunking_target(reply: &Reply) -> Result<(), MockBuildError> {
    if reply.chunking != Chunking::none() && reply.response().is_none() {
        return Err(MockBuildError::ChunkingWithoutResponse);
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

fn check_delta_count(reply: &Reply, before_completed: Option<usize>) -> Result<(), MockBuildError> {
    let (Shape::Cut { after, .. } | Shape::Hang { after, .. }) = &reply.shape else {
        return Ok(());
    };
    let available = before_completed.unwrap_or(0);
    if *after > available {
        return Err(MockBuildError::DeltaCountOutOfRange {
            requested: *after,
            available,
        });
    }
    Ok(())
}

fn sendable_deltas(reply: &Reply, before_completed: Option<usize>) -> usize {
    match &reply.shape {
        Shape::Failure(_) => 0,
        Shape::Cut { after, .. } | Shape::Hang { after, .. } => *after,
        Shape::Success(_) => before_completed.unwrap_or(0),
        Shape::Broken { deltas, .. } => deltas.len(),
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

fn check_first_delta(reply: &Reply, sendable: usize) -> Result<(), MockBuildError> {
    let sends_nothing = !matches!(reply.shape, Shape::Success(_)) && sendable == 0;
    if sends_nothing && reply.report.time_to_first_event.is_some() {
        return Err(MockBuildError::FirstEventWithoutEvent);
    }
    Ok(())
}
