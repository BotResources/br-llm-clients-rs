use br_llm_messages::{
    ClosureReason, QueuePosition, RequestAccepted, TaskCompleted, TaskFailed, TaskStarted,
    TurnClosure,
};

use super::escape;
use super::refusal::invalid;
use super::types::RenderedPart;
use crate::client_error::ClientError;

fn notice(sentence: String) -> Vec<RenderedPart> {
    vec![RenderedPart::Text(format!("<notice>{sentence}</notice>"))]
}

pub(super) fn request_accepted(entry: &RequestAccepted) -> Vec<RenderedPart> {
    notice(format!("Request {} was accepted.", entry.request()))
}

pub(super) fn queue_position(entry: &QueuePosition) -> Vec<RenderedPart> {
    notice(format!(
        "Request {} is number {} in the queue.",
        entry.request(),
        entry.position()
    ))
}

pub(super) fn task_started(entry: &TaskStarted) -> Vec<RenderedPart> {
    let mut sentence = format!(
        "Task {} started for request {}.",
        entry.task(),
        entry.request()
    );
    if let Some(conversation) = entry.conversation() {
        sentence.push_str(&format!(" It runs in conversation {conversation}."));
    }
    notice(sentence)
}

pub(super) fn task_completed(entry: &TaskCompleted) -> Vec<RenderedPart> {
    let mut sentence = format!("Task {} completed.", entry.task());
    if let Some(summary) = entry.summary() {
        sentence.push_str(&format!(" Summary: {}", escape::text(summary)));
    }
    notice(sentence)
}

pub(super) fn task_failed(entry: &TaskFailed) -> Vec<RenderedPart> {
    notice(format!(
        "Task {} failed. Reason: {}",
        entry.task(),
        escape::text(entry.reason())
    ))
}

pub(super) fn turn_closure(entry: &TurnClosure) -> Result<Vec<RenderedPart>, ClientError> {
    let (sentence, detail) = match entry.reason() {
        ClosureReason::Completed { .. } => return Ok(Vec::new()),
        ClosureReason::Cancelled { detail, .. } => (
            "Your previous turn was cancelled.",
            detail.as_ref().map(|detail| detail.as_str()),
        ),
        ClosureReason::Interrupted { detail, .. } => (
            "Your previous turn was interrupted.",
            detail.as_ref().map(|detail| detail.as_str()),
        ),
        ClosureReason::BudgetExhausted { detail, .. } => (
            "Your previous turn stopped because its budget was exhausted.",
            detail.as_ref().map(|detail| detail.as_str()),
        ),
        ClosureReason::Failed { detail, .. } => {
            ("Your previous turn failed.", Some(detail.as_str()))
        }
        _ => return Err(invalid("unknown turn closure reason")),
    };
    Ok(notice(match detail {
        Some(detail) => format!("{sentence} Detail: {}", escape::text(detail)),
        None => sentence.to_owned(),
    }))
}
