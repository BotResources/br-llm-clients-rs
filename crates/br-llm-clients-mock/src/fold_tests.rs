use br_llm_clients_core::ClientError;
use br_llm_messages::{DraftPhase, ModelResponse, ResponseDelta};

use crate::fold::{fold_deltas, fold_partial};
use crate::reply::Reply;
use crate::test_support::*;

fn served(reply: Reply) -> (ModelResponse, Vec<ResponseDelta>) {
    let client = mock();
    client.push_reply(reply);
    let (outcome, sink) = run(&client, &request());
    (
        outcome
            .result()
            .cloned()
            .unwrap_or_else(|_| text_response("cut")),
        sink.deltas(),
    )
}

fn full() -> (ModelResponse, Vec<ResponseDelta>) {
    served(Reply::text("hello").build().unwrap())
}

#[test]
fn given_no_delta_when_folded_partially_then_nothing_is_given() {
    assert!(fold_partial(&[]).is_none());
}

#[test]
fn given_all_the_deltas_when_folded_partially_then_the_draft_is_complete_and_nothing_stopped() {
    let (response, deltas) = full();
    let folded = fold_partial(&deltas).unwrap();
    assert!(folded.stop().is_none());
    assert!(matches!(
        folded.draft().phase(),
        DraftPhase::Completed { .. }
    ));
    assert_eq!(folded.into_draft().into_response().unwrap(), response);
}

#[test]
fn given_the_deltas_of_a_cut_when_folded_partially_then_the_draft_holds_what_was_sent() {
    let (_, deltas) = served(
        Reply::cut(text_response("hello"), 3, ClientError::Transport)
            .build()
            .unwrap(),
    );
    assert_eq!(deltas.len(), 3);
    let folded = fold_partial(&deltas).unwrap();
    assert!(folded.stop().is_none());
    assert!(matches!(
        folded.draft().phase(),
        DraftPhase::Producing { .. }
    ));
    assert_eq!(folded.draft().view().slots().len(), 1);
    assert!(folded.into_draft().into_response().is_none());
}

#[test]
fn given_the_first_delta_when_folded_partially_then_the_draft_is_of_its_turn_and_entry() {
    let (_, deltas) = full();
    let folded = fold_partial(&deltas).unwrap();
    assert_eq!(folded.draft().turn(), deltas[0].turn());
    assert_eq!(folded.draft().entry(), deltas[0].entry());
}

#[test]
fn given_a_delta_the_draft_refuses_when_folded_partially_then_it_stops_there_with_the_position_and_a_reason()
 {
    let (_, mut deltas) = full();
    let at = 2;
    deltas.insert(at, started_delta_of(&deltas[0]));
    let folded = fold_partial(&deltas).unwrap();
    let stop = folded.stop().unwrap();
    assert_eq!(stop.position(), at);
    assert!(!stop.reason().is_empty());
    assert_eq!(folded.draft().view().slots().len(), 1);
    let before = fold_partial(&deltas[..at]).unwrap();
    assert_eq!(before.draft().view().slots().len(), 1);
}

#[test]
fn given_a_refused_delta_when_folded_partially_then_the_deltas_after_it_are_not_applied() {
    let (_, mut deltas) = full();
    deltas.insert(1, started_delta_of(&deltas[0]));
    let folded = fold_partial(&deltas).unwrap();
    assert_eq!(folded.stop().unwrap().position(), 1);
    assert_eq!(folded.draft().view().slots().len(), 0);
}

#[test]
fn given_all_the_deltas_when_folded_then_the_response_the_call_gave() {
    let (response, deltas) = full();
    assert_eq!(fold_deltas(&deltas).unwrap(), response);
}

#[test]
fn given_no_delta_when_folded_then_an_error_with_a_reason() {
    assert!(!fold_deltas(&[]).unwrap_err().reason().is_empty());
}

#[test]
fn given_deltas_that_stop_before_completed_when_folded_then_an_error() {
    let (_, deltas) = full();
    let error = fold_deltas(&deltas[..deltas.len() - 1]).unwrap_err();
    assert!(!error.reason().is_empty());
}

#[test]
fn given_a_refused_delta_when_folded_then_the_error_names_its_position() {
    let (_, mut deltas) = full();
    deltas.insert(2, started_delta_of(&deltas[0]));
    let error = fold_deltas(&deltas).unwrap_err();
    assert!(error.reason().contains("delta 2"));
}

#[test]
fn given_the_deltas_of_two_calls_when_folded_then_an_error() {
    let (_, mut first) = full();
    let (_, second) = full();
    first.extend(second);
    assert!(fold_deltas(&first).is_err());
}

fn started_delta_of(first: &ResponseDelta) -> ResponseDelta {
    ResponseDelta::new(
        first.turn(),
        first.entry(),
        br_llm_messages::DeltaOp::response_started(br_llm_messages::ResponseMetadata::new()),
    )
}
