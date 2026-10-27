use br_llm_messages::{IncomingEntry, TurnEntry};

use super::content;
use super::frames::{self, AGENT_MAIL, AGENT_OUTPUT, AGENT_REQUEST, PARTIAL_AGENT_OUTPUT};
use super::notices;
use super::refusal::invalid;
use super::types::RenderedPart;
use crate::client_error::ClientError;

pub(super) fn of_incoming(entry: &IncomingEntry) -> Result<Vec<RenderedPart>, ClientError> {
    match entry {
        IncomingEntry::HumanMessage(message) => content::plain_parts(message.content()),
        IncomingEntry::AgentRequest(request) => {
            frames::framed(AGENT_REQUEST, request.author(), request.content())
        }
        IncomingEntry::AgentMail(mail) => frames::framed(AGENT_MAIL, mail.author(), mail.content()),
        IncomingEntry::AgentOutput(output) => {
            frames::framed(AGENT_OUTPUT, output.author(), output.content())
        }
        IncomingEntry::PartialAgentOutput(output) => {
            frames::framed(PARTIAL_AGENT_OUTPUT, output.author(), output.content())
        }
        IncomingEntry::RequestAccepted(entry) => Ok(notices::request_accepted(entry)),
        IncomingEntry::QueuePosition(entry) => Ok(notices::queue_position(entry)),
        IncomingEntry::TaskStarted(entry) => Ok(notices::task_started(entry)),
        IncomingEntry::TaskCompleted(entry) => Ok(notices::task_completed(entry)),
        IncomingEntry::TaskFailed(entry) => Ok(notices::task_failed(entry)),
        other => Err(invalid(match other.kind() {
            Some(kind) => format!("unknown incoming entry kind {}", kind.as_str()),
            None => "unknown incoming entry".to_owned(),
        })),
    }
}

pub(super) fn of_turn_entry(entry: &TurnEntry) -> Result<Option<Vec<RenderedPart>>, ClientError> {
    Ok(Some(match entry {
        TurnEntry::HumanMessage(message) => content::plain_parts(message.content())?,
        TurnEntry::AgentRequest(request) => {
            frames::framed(AGENT_REQUEST, request.author(), request.content())?
        }
        TurnEntry::AgentMail(mail) => frames::framed(AGENT_MAIL, mail.author(), mail.content())?,
        TurnEntry::RequestAccepted(entry) => notices::request_accepted(entry),
        TurnEntry::QueuePosition(entry) => notices::queue_position(entry),
        TurnEntry::TaskStarted(entry) => notices::task_started(entry),
        TurnEntry::TaskCompleted(entry) => notices::task_completed(entry),
        TurnEntry::TaskFailed(entry) => notices::task_failed(entry),
        _ => return Ok(None),
    }))
}
