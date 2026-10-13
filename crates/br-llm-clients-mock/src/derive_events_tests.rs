use br_llm_messages::{
    AssistantBlock, BlockKind, RedactedThinking, Signature, Step, StepDraft, StopReason,
    StreamEvent, Thinking, ToolCallId, ToolName,
};
use serde_json::json;

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

fn rich_step() -> Step {
    Step::new(
        vec![
            thinking("pondering the question", Some("sig-123")),
            AssistantBlock::RedactedThinking(RedactedThinking::new("opaque").unwrap()),
            text_block("héllo wörld, ça va ?"),
            AssistantBlock::Structured {
                value: json!({ "answer": [1, 2.5, null, "é"], "ok": true }),
            },
            tool_call("call-1", "search", json!({ "q": "rust", "n": 3 })),
            tool_call("call-2", "search", json!({})),
        ],
        StopReason::AwaitingToolResults,
        Some(usage(10, 20)),
        None,
    )
    .unwrap()
}

fn fold(events: Vec<StreamEvent>, step_value: &Step) -> Step {
    let mut draft = StepDraft::new();
    for event in events {
        draft.apply(event).unwrap();
    }
    draft.finish(step_value.model().cloned()).unwrap()
}

#[test]
fn given_a_text_block_when_derived_then_start_one_delta_and_end() {
    let derived = derive_events(&text_step("hello"), &Chunking::none()).unwrap();
    assert_eq!(
        derived.events(),
        [
            StreamEvent::BlockStart {
                index: 0,
                kind: BlockKind::Text
            },
            StreamEvent::TextDelta {
                index: 0,
                text: "hello".to_owned()
            },
            StreamEvent::BlockEnd { index: 0 },
            StreamEvent::Finish {
                stop_reason: StopReason::EndTurn,
                usage: None
            },
        ]
    );
    assert_eq!(derived.before_finish(), 3);
}

#[test]
fn given_a_thinking_block_with_a_signature_when_derived_then_the_signature_follows_the_deltas() {
    let step_value = step(vec![thinking("abcd", Some("sig"))], StopReason::EndTurn);
    let derived = derive_events(&step_value, &Chunking::none().thinking(2)).unwrap();
    assert_eq!(
        &derived.events()[..5],
        [
            StreamEvent::BlockStart {
                index: 0,
                kind: BlockKind::Thinking
            },
            StreamEvent::ThinkingDelta {
                index: 0,
                text: "ab".to_owned()
            },
            StreamEvent::ThinkingDelta {
                index: 0,
                text: "cd".to_owned()
            },
            StreamEvent::SignatureDelta {
                index: 0,
                signature: "sig".to_owned()
            },
            StreamEvent::BlockEnd { index: 0 },
        ]
    );
}

#[test]
fn given_a_thinking_block_with_an_empty_text_when_derived_then_it_gives_no_thinking_delta() {
    let step_value = step(vec![thinking("", Some("sig"))], StopReason::EndTurn);
    let derived = derive_events(&step_value, &Chunking::none()).unwrap();
    assert!(
        !derived
            .events()
            .iter()
            .any(|event| matches!(event, StreamEvent::ThinkingDelta { .. }))
    );
    assert_eq!(fold(derived.into_events(), &step_value), step_value);
}

#[test]
fn given_a_redacted_block_when_derived_then_one_data_event_that_is_never_cut() {
    let step_value = step(
        vec![AssistantBlock::RedactedThinking(
            RedactedThinking::new("abcdefgh").unwrap(),
        )],
        StopReason::EndTurn,
    );
    let derived = derive_events(&step_value, &Chunking::none().text(1).thinking(1)).unwrap();
    assert_eq!(
        derived.events()[1],
        StreamEvent::RedactedThinkingData {
            index: 0,
            data: "abcdefgh".to_owned()
        }
    );
    assert_eq!(derived.events().len(), 4);
}

#[test]
fn given_a_structured_block_when_derived_then_deltas_cut_the_compact_json_text() {
    let step_value = step(
        vec![AssistantBlock::Structured {
            value: json!({ "a": 1 }),
        }],
        StopReason::EndTurn,
    );
    let derived = derive_events(&step_value, &Chunking::none().structured(3)).unwrap();
    let fragments: Vec<_> = derived
        .events()
        .iter()
        .filter_map(|event| match event {
            StreamEvent::StructuredDelta { json_fragment, .. } => Some(json_fragment.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(fragments, ["{\"a", "\":1", "}"]);
}

#[test]
fn given_a_tool_call_when_derived_then_the_start_carries_id_and_name() {
    let step_value = step(
        vec![tool_call("c1", "search", json!({ "q": "x" }))],
        StopReason::AwaitingToolResults,
    );
    let derived = derive_events(&step_value, &Chunking::none()).unwrap();
    assert_eq!(
        derived.events()[0],
        StreamEvent::ToolCallStart {
            index: 0,
            id: ToolCallId::new("c1").unwrap(),
            name: ToolName::new("search").unwrap()
        }
    );
    assert_eq!(
        derived.events()[1],
        StreamEvent::ToolCallArgumentsDelta {
            index: 0,
            json_fragment: "{\"q\":\"x\"}".to_owned()
        }
    );
}

#[test]
fn given_empty_arguments_when_derived_then_the_text_is_braces() {
    let step_value = step(
        vec![tool_call("c1", "search", json!({}))],
        StopReason::AwaitingToolResults,
    );
    let derived = derive_events(&step_value, &Chunking::none()).unwrap();
    assert_eq!(
        derived.events()[1],
        StreamEvent::ToolCallArgumentsDelta {
            index: 0,
            json_fragment: "{}".to_owned()
        }
    );
}

#[test]
fn given_several_blocks_when_derived_then_indexes_are_consecutive_from_zero_in_step_order() {
    let derived = derive_events(&rich_step(), &Chunking::none()).unwrap();
    let mut starts = Vec::new();
    for event in derived.events() {
        match event {
            StreamEvent::BlockStart { index, .. } | StreamEvent::ToolCallStart { index, .. } => {
                starts.push(*index)
            }
            _ => {}
        }
    }
    assert_eq!(starts, [0, 1, 2, 3, 4, 5]);
}

#[test]
fn given_the_finish_event_when_derived_then_it_is_last_with_the_stop_reason_and_usage() {
    let derived = derive_events(&rich_step(), &Chunking::none()).unwrap();
    assert_eq!(
        derived.events().last(),
        Some(&StreamEvent::Finish {
            stop_reason: StopReason::AwaitingToolResults,
            usage: Some(usage(10, 20))
        })
    );
    assert_eq!(derived.before_finish(), derived.events().len() - 1);
}

#[test]
fn given_any_chunking_when_folded_then_the_events_give_back_the_step() {
    let step_value = rich_step();
    for chunking in [
        Chunking::none(),
        Chunking::none().text(1),
        Chunking::none()
            .text(3)
            .thinking(2)
            .tool_arguments(1)
            .structured(4),
        Chunking::none()
            .text(1000)
            .thinking(1000)
            .tool_arguments(1000)
            .structured(1000),
    ] {
        let derived = derive_events(&step_value, &chunking).unwrap();
        assert_eq!(fold(derived.into_events(), &step_value), step_value);
    }
}

#[test]
fn given_a_step_with_a_model_when_folded_with_that_model_then_the_steps_are_equal() {
    let step_value = Step::new(
        vec![text_block("hi")],
        StopReason::EndTurn,
        None,
        Some(model("m-1")),
    )
    .unwrap();
    let derived = derive_events(&step_value, &Chunking::none()).unwrap();
    assert_eq!(fold(derived.into_events(), &step_value), step_value);
}

#[test]
fn given_a_blank_only_piece_when_derived_then_it_is_sent_like_any_other_in_order() {
    let step_value = step(vec![text_block(" a  b ")], StopReason::EndTurn);
    let derived = derive_events(&step_value, &Chunking::none().text(1)).unwrap();
    let joined: String = derived
        .events()
        .iter()
        .filter_map(|event| match event {
            StreamEvent::TextDelta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(joined, " a  b ");
}
