use br_llm_messages::{AgentMail, AgentRequest, HumanMessage, IncomingEntry, TurnEntry};

use super::content;
use super::frames::{self, AGENT_MAIL, AGENT_OUTPUT, AGENT_REQUEST, PARTIAL_AGENT_OUTPUT};
use super::notices;
use super::refusal::invalid;
use super::types::RenderedPart;
use crate::client_error::ClientError;

pub(super) fn of_incoming(entry: &IncomingEntry) -> Result<Vec<RenderedPart>, ClientError> {
    match entry {
        IncomingEntry::HumanMessage(message) => human_message(message),
        IncomingEntry::AgentRequest(request) => agent_request(request),
        IncomingEntry::AgentMail(mail) => agent_mail(mail),
        IncomingEntry::AgentOutput(output) => {
            frames::framed(AGENT_OUTPUT, output.author(), output.content())
        }
        IncomingEntry::PartialAgentOutput(output) => {
            frames::framed(PARTIAL_AGENT_OUTPUT, output.author(), output.content())
        }
        IncomingEntry::RuntimeNotice(entry) => Ok(notices::runtime_notice(entry)),
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
        TurnEntry::HumanMessage(message) => human_message(message)?,
        TurnEntry::AgentRequest(request) => agent_request(request)?,
        TurnEntry::AgentMail(mail) => agent_mail(mail)?,
        TurnEntry::RuntimeNotice(entry) => notices::runtime_notice(entry),
        TurnEntry::RequestAccepted(entry) => notices::request_accepted(entry),
        TurnEntry::QueuePosition(entry) => notices::queue_position(entry),
        TurnEntry::TaskStarted(entry) => notices::task_started(entry),
        TurnEntry::TaskCompleted(entry) => notices::task_completed(entry),
        TurnEntry::TaskFailed(entry) => notices::task_failed(entry),
        _ => return Ok(None),
    }))
}

fn human_message(message: &HumanMessage) -> Result<Vec<RenderedPart>, ClientError> {
    content::plain_parts(message.content())
}

fn agent_request(request: &AgentRequest) -> Result<Vec<RenderedPart>, ClientError> {
    frames::framed(AGENT_REQUEST, request.author(), request.content())
}

fn agent_mail(mail: &AgentMail) -> Result<Vec<RenderedPart>, ClientError> {
    frames::framed(AGENT_MAIL, mail.author(), mail.content())
}
