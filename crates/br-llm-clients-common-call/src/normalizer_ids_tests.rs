use std::sync::Arc;

use br_llm_messages::{BlockStart, DeltaOp, ResponseDelta};

use crate::normalizer::Normalizer;
use crate::test_decoder::script;
use crate::test_fixtures::user_request;
use crate::test_ids::{CountingIds, number_of};
use crate::test_normalize::{feed, fresh, refusal};

fn starts(deltas: &[ResponseDelta]) -> Vec<(u64, Option<String>)> {
    deltas
        .iter()
        .filter_map(|delta| match delta.op() {
            DeltaOp::BlockStarted { block, start, .. } => {
                let external = match start {
                    BlockStart::RuntimeToolCall { external_id, .. } => {
                        external_id.as_ref().map(|id| id.as_str().to_owned())
                    }
                    _ => None,
                };
                Some((number_of(*block), external))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn given_an_id_from_the_provider_when_started_then_it_is_the_external_id_exactly_as_received() {
    let deltas = feed(&mut fresh(), &["start a tool search Call-Id.7:x"]);
    assert_eq!(starts(&deltas), [(1, Some("Call-Id.7:x".to_owned()))]);
}

#[test]
fn given_no_id_or_an_empty_id_when_started_then_the_call_has_no_external_id() {
    let deltas = feed(
        &mut fresh(),
        &["start a tool search", "end a", "start b tool search ~"],
    );
    assert_eq!(starts(&deltas), [(1, None), (2, None)]);
}

#[test]
fn given_an_id_with_the_old_generated_prefix_when_started_then_it_is_carried_as_received() {
    let deltas = feed(&mut fresh(), &["start a tool search brgen_1"]);
    assert_eq!(starts(&deltas), [(1, Some("brgen_1".to_owned()))]);
}

#[test]
fn given_a_provider_id_used_twice_in_the_reply_when_started_then_both_calls_carry_it() {
    let deltas = feed(
        &mut fresh(),
        &[
            "start a tool search dup",
            "args a {}",
            "end a",
            "start b tool search dup",
            "args b {}",
            "end b",
        ],
    );
    assert_eq!(
        starts(&deltas),
        [(1, Some("dup".to_owned())), (2, Some("dup".to_owned()))]
    );
}

#[test]
fn given_an_id_source_when_blocks_are_sent_then_the_ids_come_from_it_in_the_order_they_are_sent() {
    let ids = Arc::new(CountingIds::from(40));
    let mut normalizer = Normalizer::new(&user_request(), ids.clone(), None);
    let lines = [
        "start a text",
        "start b tool search x",
        "start c thinking",
        "think c t",
        "args b {}",
        "text a Hi",
        "end a",
        "end b",
        "end c",
    ];
    let deltas: Vec<_> = script(&lines)
        .into_iter()
        .flat_map(|event| normalizer.accept(event).expect("accepted"))
        .collect();
    let numbers: Vec<_> = starts(&deltas).into_iter().map(|start| start.0).collect();
    assert_eq!(numbers, [40, 41, 42]);
    assert_eq!(ids.taken(), 3);
}

#[test]
fn given_blocks_that_are_not_sent_when_the_stream_runs_then_no_id_is_taken() {
    let ids = Arc::new(CountingIds::new());
    let mut normalizer = Normalizer::new(&user_request(), ids.clone(), None);
    let lines = [
        "start a text",
        "text a  ",
        "end a",
        "start b thinking",
        "end b",
    ];
    for event in script(&lines) {
        normalizer.accept(event).expect("accepted");
    }
    assert_eq!(ids.taken(), 0);
}

#[test]
fn given_a_block_waiting_behind_an_open_one_when_nothing_is_sent_for_it_then_no_id_is_taken_yet() {
    let ids = Arc::new(CountingIds::new());
    let mut normalizer = Normalizer::new(&user_request(), ids.clone(), None);
    for event in script(&["start a text", "start b tool search x", "args b {}"]) {
        normalizer.accept(event).expect("accepted");
    }
    assert_eq!(ids.taken(), 0);
}

#[test]
fn given_the_rehearsal_of_the_tail_when_blocks_are_closed_then_no_id_is_taken_from_the_source() {
    let ids = Arc::new(CountingIds::new());
    let mut normalizer = Normalizer::new(&user_request(), ids.clone(), None);
    let lines = ["start a text", "text a  ", "start b thinking", "think b t"];
    for event in script(&lines) {
        normalizer.accept(event).expect("accepted");
    }
    let mut rehearsal = normalizer.rehearsal();
    assert_eq!(starts(&rehearsal.close_all()).len(), 1);
    assert_eq!(ids.taken(), 0);
    assert_eq!(starts(&normalizer.close_all()), [(1, None)]);
    assert_eq!(ids.taken(), 1);
}

#[test]
fn given_a_tool_name_that_is_not_valid_when_started_then_the_reply_is_malformed() {
    let error = refusal(&[], "start a tool bad.name id1");
    assert!(matches!(
        error,
        br_llm_clients_core::ClientError::MalformedResponse { .. }
    ));
}
