use br_llm_messages::{
    ContentBlock, DeltaOp, ModelResponse, OpaqueJson, ReplayFormat, ReplayMaterial, ReplayPayload,
    ResponseEnding, ResponseMetadata, ThinkingBlock,
};

use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::derive_deltas::derive_deltas;
use crate::fixtures::{block_id, turn_id};
use crate::test_support::*;

fn derived(response: &ModelResponse, chunking: &Chunking) -> Vec<DeltaOp> {
    derive_deltas(turn_id(), response, chunking)
        .unwrap()
        .into_deltas()
        .iter()
        .map(|delta| delta.op().clone())
        .collect()
}

fn texts(ops: &[DeltaOp]) -> Vec<String> {
    ops.iter()
        .filter_map(|op| match op {
            DeltaOp::TextAppended { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn json_fragments(ops: &[DeltaOp]) -> Vec<String> {
    ops.iter()
        .filter_map(|op| match op {
            DeltaOp::JsonAppended { fragment, .. } => Some(fragment.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn given_replay_material_when_thinking_is_cut_then_the_material_stays_whole() {
    let material = ReplayMaterial::new(
        ReplayFormat::new("sig").unwrap(),
        ReplayPayload::text("longsignature"),
    );
    let block = ThinkingBlock::new(block_id(), "xy")
        .unwrap()
        .with_replay(material.clone());
    let response = response_of(
        vec![ContentBlock::Thinking(block)],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    );
    let ops = derived(&response, &Chunking::none().thinking(1));
    let thinking = ops
        .iter()
        .filter(|op| matches!(op, DeltaOp::ThinkingAppended { .. }))
        .count();
    let replays: Vec<_> = ops
        .iter()
        .filter_map(|op| match op {
            DeltaOp::ReplayMaterialSet { material, .. } => Some(material.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(thinking, 2);
    assert_eq!(replays, [material]);
}

#[test]
fn given_text_with_multibyte_characters_when_cut_then_no_scalar_value_is_split_and_sizes_are_exact()
{
    let response = text_response("aé😀bç😀d");
    let ops = derived(&response, &Chunking::none().text(3));
    assert_eq!(texts(&ops), ["aé😀", "bç😀", "d"]);
}

#[test]
fn given_a_size_larger_than_the_text_when_cut_then_one_piece() {
    let ops = derived(&text_response("abc"), &Chunking::none().text(10));
    assert_eq!(texts(&ops), ["abc"]);
}

#[test]
fn given_no_size_when_derived_then_a_text_is_one_piece() {
    let ops = derived(&text_response("abcdef"), &Chunking::none());
    assert_eq!(texts(&ops), ["abcdef"]);
}

#[test]
fn given_sizes_per_kind_when_cut_then_each_kind_uses_its_own_size_and_others_stay_whole() {
    let response = response_of(
        vec![
            text_content("abcdef"),
            call_content("search", r#"{"key":"value"}"#),
        ],
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    );
    let ops = derived(&response, &Chunking::none().tool_arguments(5));
    assert_eq!(texts(&ops), ["abcdef"]);
    assert_eq!(json_fragments(&ops), [r#"{"key"#, r#"":"va"#, r#"lue"}"#]);
}

#[test]
fn given_a_structured_value_when_cut_then_the_pieces_follow_its_compact_json_text() {
    let value = OpaqueJson::parse(r#"{ "a" : [1, 2] }"#).unwrap();
    let block = br_llm_messages::StructuredBlock::new(block_id(), value).unwrap();
    let response = response_of(
        vec![ContentBlock::StructuredContent(block)],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    );
    let ops = derived(&response, &Chunking::none().structured(4));
    assert_eq!(json_fragments(&ops), [r#"{"a""#, r#":[1,"#, "2]}"]);
}

#[test]
fn given_arguments_of_an_empty_object_when_derived_then_the_text_is_two_braces() {
    let response = response_of(
        vec![call_content("search", "{}")],
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    );
    let ops = derived(&response, &Chunking::none());
    assert_eq!(json_fragments(&ops), ["{}"]);
}

#[test]
fn given_a_thinking_block_with_an_empty_text_when_derived_then_it_gives_no_thinking_delta() {
    let material = ReplayMaterial::new(
        ReplayFormat::new("sig").unwrap(),
        ReplayPayload::text("only"),
    );
    let block = ThinkingBlock::replay_only(block_id(), material);
    let response = response_of(
        vec![ContentBlock::Thinking(block), text_content("a")],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    );
    let ops = derived(&response, &Chunking::none());
    assert!(
        !ops.iter()
            .any(|op| matches!(op, DeltaOp::ThinkingAppended { .. }))
    );
}

#[test]
fn given_a_size_of_zero_when_derived_then_chunk_size_zero() {
    for chunking in [
        Chunking::none().text(0),
        Chunking::none().thinking(0),
        Chunking::none().tool_arguments(0),
        Chunking::none().structured(0),
    ] {
        assert_eq!(
            derive_deltas(turn_id(), &text_response("a"), &chunking).map(|_| ()),
            Err(MockBuildError::ChunkSizeZero)
        );
    }
}
