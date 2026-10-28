use br_llm_messages::{ModelResponse, ResponseDeltaKind as K, ResponseEnding, ResponseMetadata};
use serde_json::Value;

use crate::chunking::Chunking;
use crate::derive_deltas::derive_deltas;
use crate::fold::fold_deltas;
use crate::maximal::maximal;
use crate::test_support::*;

fn derived(response: &ModelResponse) -> Vec<br_llm_messages::ResponseDelta> {
    derive_deltas(turn_of_tests(), response, &Chunking::none())
        .unwrap()
        .into_deltas()
}

fn turn_of_tests() -> br_llm_messages::TurnId {
    crate::fixtures::turn_id()
}

fn kinds(deltas: &[br_llm_messages::ResponseDelta]) -> Vec<K> {
    deltas.iter().map(|delta| delta.kind()).collect()
}

fn start_of(delta: &br_llm_messages::ResponseDelta) -> Value {
    serde_json::to_value(delta).unwrap()
}

#[test]
fn given_a_response_with_every_kind_of_block_when_derived_then_the_deltas_follow_the_order_of_the_text()
 {
    let built = maximal();
    let deltas = derived(&built.response);
    let expected = [
        vec![K::ResponseStarted],
        vec![
            K::BlockStarted,
            K::ThinkingAppended,
            K::ReplayMaterialSet,
            K::BlockFinalized,
        ],
        vec![K::BlockStarted, K::ReplayMaterialSet, K::BlockFinalized],
        vec![
            K::BlockStarted,
            K::TextAppended,
            K::BlockMetadataUpdated,
            K::BlockMetadataUpdated,
            K::ReplayMaterialSet,
            K::BlockFinalized,
        ],
        vec![K::BlockInserted],
        vec![
            K::BlockStarted,
            K::JsonAppended,
            K::ReplayMaterialSet,
            K::BlockFinalized,
        ],
        vec![K::BlockInserted, K::BlockInserted, K::BlockInserted],
        vec![K::ResponseMetadataUpdated, K::ResponseCompleted],
    ]
    .concat();
    assert_eq!(kinds(&deltas), expected);
}

#[test]
fn given_a_response_when_derived_then_every_delta_names_the_turn_given_and_the_entry_of_the_response()
 {
    let built = maximal();
    let turn = turn_of_tests();
    let deltas = derive_deltas(turn, &built.response, &Chunking::none())
        .unwrap()
        .into_deltas();
    for delta in &deltas {
        assert_eq!(delta.turn(), turn);
        assert_eq!(delta.entry(), built.response.id());
    }
}

#[test]
fn given_a_response_when_derived_then_the_deltas_fold_to_the_response_with_the_corrected_ending() {
    let built = maximal();
    let deltas = derived(&built.response);
    assert_eq!(fold_deltas(&deltas).unwrap(), built.response);
}

#[test]
fn given_a_response_with_model_provider_id_and_time_when_derived_then_response_started_carries_them()
 {
    let built = maximal();
    let started = start_of(&derived(&built.response)[0]).to_string();
    assert!(started.contains("answering"));
    assert!(started.contains("resp_1"));
    assert!(started.contains("2026-10-27T10:00:00Z"));
}

#[test]
fn given_a_response_with_no_metadata_when_derived_then_response_started_is_still_first_and_no_update_follows()
 {
    let deltas = derived(&text_response("hi"));
    assert_eq!(
        kinds(&deltas),
        [
            K::ResponseStarted,
            K::BlockStarted,
            K::TextAppended,
            K::BlockFinalized,
            K::ResponseCompleted
        ]
    );
}

#[test]
fn given_a_usage_and_no_block_when_derived_then_the_update_follows_response_started() {
    let response = ModelResponse::new(
        crate::fixtures::entry_id(),
        vec![],
        ResponseEnding::finished(),
        ResponseMetadata::new().with_usage(usage(1, 2)),
    )
    .unwrap();
    assert_eq!(
        kinds(&derived(&response)),
        [
            K::ResponseStarted,
            K::ResponseMetadataUpdated,
            K::ResponseCompleted
        ]
    );
}

#[test]
fn given_a_usage_when_derived_then_the_update_carries_the_whole_usage_after_the_last_block() {
    let response = text_response_with_usage("hi", usage(3, 4));
    let deltas = derived(&response);
    let at = deltas.len() - 2;
    assert_eq!(deltas[at].kind(), K::ResponseMetadataUpdated);
    assert!(start_of(&deltas[at]).to_string().contains('4'));
}

#[test]
fn given_a_text_block_with_an_item_ref_and_a_time_when_derived_then_its_start_carries_them() {
    let built = maximal();
    let deltas = derived(&built.response);
    let text_start = deltas
        .iter()
        .find(|delta| {
            delta.kind() == K::BlockStarted && start_of(delta).to_string().contains("item_t")
        })
        .unwrap();
    assert!(
        start_of(text_start)
            .to_string()
            .contains("2026-10-27T10:00:00Z")
    );
    let call_start = deltas
        .iter()
        .find(|delta| {
            delta.kind() == K::BlockStarted && start_of(delta).to_string().contains("item_rc")
        })
        .unwrap();
    let call_json = start_of(call_start).to_string();
    assert!(call_json.contains("search"));
    assert!(call_json.contains("rc_1"));
}

#[test]
fn given_a_response_with_a_runtime_call_and_the_ending_tool_use_when_derived_then_completed_carries_tool_use()
 {
    let response = response_of(
        vec![call_content("search", "{}")],
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    );
    let last = derived(&response).pop().unwrap();
    assert_eq!(last.kind(), K::ResponseCompleted);
    assert!(start_of(&last).to_string().contains("tool_use"));
}

#[test]
fn given_a_response_with_a_runtime_call_and_another_ending_when_derived_then_completed_carries_tool_use()
 {
    for ending in [
        ResponseEnding::finished(),
        ResponseEnding::refusal(),
        ResponseEnding::output_exhaustion(),
    ] {
        let response = response_of(
            vec![call_content("search", "{}")],
            ending,
            ResponseMetadata::new(),
        );
        let last = derived(&response).pop().unwrap();
        assert_eq!(last.kind(), K::ResponseCompleted);
        let json = start_of(&last).to_string();
        assert!(json.contains("tool_use"), "{json}");
    }
}

#[test]
fn given_a_response_with_a_runtime_call_and_the_ending_finished_when_derived_then_the_deltas_fold_to_a_tool_use_response()
 {
    let blocks = vec![text_content("let me look"), call_content("search", "{}")];
    let response = response_of(
        blocks.clone(),
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    );
    let folded = fold_deltas(&derived(&response)).unwrap();
    assert_eq!(folded.ending(), &ResponseEnding::tool_use());
    assert_eq!(folded.blocks(), blocks.as_slice());
}

#[test]
fn given_a_response_with_no_runtime_call_when_derived_then_completed_keeps_the_ending_of_the_response()
 {
    let response = response_of(
        vec![text_content("hi")],
        ResponseEnding::output_exhaustion(),
        ResponseMetadata::new(),
    );
    let folded = fold_deltas(&derived(&response)).unwrap();
    assert_eq!(folded.ending(), &ResponseEnding::output_exhaustion());
}

#[test]
fn given_the_derivation_when_asked_for_the_count_then_it_is_the_length_minus_the_completed_delta() {
    let built = maximal();
    let derived = derive_deltas(turn_of_tests(), &built.response, &Chunking::none()).unwrap();
    assert_eq!(derived.before_completed(), derived.deltas().len() - 1);
    assert_eq!(
        derived.deltas().last().unwrap().kind(),
        K::ResponseCompleted
    );
}

#[test]
fn given_two_different_turns_when_derived_then_the_count_is_the_same() {
    let built = maximal();
    let first = derive_deltas(turn_of_tests(), &built.response, &Chunking::none()).unwrap();
    let second = derive_deltas(turn_of_tests(), &built.response, &Chunking::none()).unwrap();
    assert_eq!(first.before_completed(), second.before_completed());
}
