use br_llm_messages::{AITurn, ProjectedThread, Turn, TurnEntry};

use super::calls::CallTable;
use super::incoming;
use super::merge::{Item, Merge};
use super::notices;
use super::refusal::invalid;
use super::response;
use super::results;
use super::types::RenderedMessage;
use crate::client_error::ClientError;
use br_llm_messages::TurnId;

pub(super) fn messages(
    thread: &ProjectedThread,
    reply_turn: TurnId,
    calls: &CallTable,
) -> Result<Vec<RenderedMessage>, ClientError> {
    let mut merge = Merge::default();
    for turn in thread.turns() {
        match turn {
            Turn::InputTurn(input) => {
                for entry in input.entries() {
                    merge.push(Item::user(incoming::of_incoming(entry)?));
                }
            }
            Turn::AITurn(ai) => ai_turn(ai, ai.id() == reply_turn, calls, &mut merge)?,
            other => {
                return Err(invalid(format!(
                    "unknown turn kind {}",
                    other.kind().as_str()
                )));
            }
        }
    }
    Ok(merge.finish())
}

fn ai_turn(
    turn: &AITurn,
    reply: bool,
    calls: &CallTable,
    merge: &mut Merge,
) -> Result<(), ClientError> {
    let state = turn.state();
    if !state.is_closed()
        && let Some(call) = state.outstanding_runtime_calls().first()
    {
        return Err(invalid(format!(
            "tool call {call} has no result and its turn is not closed"
        )));
    }
    for entry in turn.entries() {
        match entry {
            TurnEntry::ModelResponse(model) => {
                merge.push(Item::assistant(response::parts(model, reply, calls)?));
            }
            TurnEntry::RuntimeToolResults(delivery) => {
                merge.push(results::delivered(delivery, calls)?);
            }
            TurnEntry::TurnClosure(closure) => {
                merge.push(results::unresolved(
                    state.outstanding_runtime_calls(),
                    calls,
                )?);
                merge.push(Item::user(notices::turn_closure(closure)?));
            }
            other => match incoming::of_turn_entry(other)? {
                Some(parts) => merge.push(Item::user(parts)),
                None => {
                    return Err(invalid(format!(
                        "unknown entry kind {}",
                        other.kind().as_str()
                    )));
                }
            },
        }
    }
    Ok(())
}
