use br_llm_messages::{AssistantBlock, Signature, StopReason, StreamEvent, Thinking};
use serde_json::json;

use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::derive_events::derive_events;
use crate::test_support::*;

fn thinking(text_value: &str, signature: Option<&str>) -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: text_value.to_owned(),
        signature: signature.map(|value| Signature::new(value).unwrap()),
    })
}

fn text_block(value: &str) -> AssistantBlock {
    AssistantBlock::Text { text: text(value) }
}

#[test]
fn given_a_signature_when_thinking_is_cut_then_the_signature_stays_whole() {
    let step_value = step(
        vec![thinking("xy", Some("longsignature"))],
        StopReason::EndTurn,
    );
    let derived = derive_events(&step_value, &Chunking::none().thinking(1)).unwrap();
    let signatures: Vec<_> = derived
        .events()
        .iter()
        .filter(|event| matches!(event, StreamEvent::SignatureDelta { .. }))
        .collect();
    assert_eq!(signatures.len(), 1);
}

#[test]
fn given_text_with_multibyte_characters_when_cut_then_no_scalar_value_is_split_and_sizes_are_exact()
{
    let step_value = step(vec![text_block("aé😀bç😀d")], StopReason::EndTurn);
    let derived = derive_events(&step_value, &Chunking::none().text(3)).unwrap();
    let pieces: Vec<&str> = derived
        .events()
        .iter()
        .filter_map(|event| match event {
            StreamEvent::TextDelta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(pieces, ["aé😀", "bç😀", "d"]);
}

#[test]
fn given_a_size_larger_than_the_text_when_cut_then_one_piece() {
    let derived = derive_events(&text_step("abc"), &Chunking::none().text(10)).unwrap();
    assert_eq!(derived.before_finish(), 3);
}

#[test]
fn given_sizes_per_kind_when_cut_then_each_kind_uses_its_own_size_and_others_stay_whole() {
    let step_value = step(
        vec![
            text_block("abcdef"),
            tool_call("c1", "search", json!({ "key": "value" })),
        ],
        StopReason::AwaitingToolResults,
    );
    let derived = derive_events(&step_value, &Chunking::none().tool_arguments(5)).unwrap();
    let text_deltas = derived
        .events()
        .iter()
        .filter(|event| matches!(event, StreamEvent::TextDelta { .. }))
        .count();
    let argument_deltas = derived
        .events()
        .iter()
        .filter(|event| matches!(event, StreamEvent::ToolCallArgumentsDelta { .. }))
        .count();
    assert_eq!(text_deltas, 1);
    assert_eq!(argument_deltas, 3);
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
            derive_events(&text_step("a"), &chunking),
            Err(MockBuildError::ChunkSizeZero)
        );
    }
}
