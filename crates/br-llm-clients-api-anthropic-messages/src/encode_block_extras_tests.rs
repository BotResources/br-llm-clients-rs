use br_llm_clients_core::{RenderedPart, render_history};
use br_llm_messages::{ContentBlock, RuntimeToolCall, TextBlock};
use serde_json::{Value, json};

use crate::test_fixtures::*;
use crate::test_thread::*;

fn body_with(blocks: Vec<ContentBlock>, call: &RuntimeToolCall) -> (Value, Vec<RenderedPart>) {
    let mut response = blocks;
    response.push(call_block(call));
    let request = request(vec![
        user("go"),
        in_progress(vec![
            calling(response),
            results(vec![result_of(call, texts(&["r"]), false)]),
        ]),
    ]);
    let parts = render_history(&request).expect("a rendering").messages()[1]
        .parts()
        .to_vec();
    (body_of(&dialect(), &request), parts)
}

fn plain_call() -> RuntimeToolCall {
    call("t1", "lookup", json!({"q": 1}))
}

fn assistant_content(body: &Value) -> &Value {
    &body["messages"][1]["content"]
}

#[test]
fn given_a_tool_call_with_replay_material_in_the_turn_in_progress_when_encoded_then_the_material_is_not_sent()
 {
    let call = plain_call().with_replay(material("other/format", "call-replay-secret"));
    let (body, parts) = body_with(Vec::new(), &call);
    assert!(matches!(&parts[0], RenderedPart::ToolCall { call, .. } if call.replay().is_some()));
    assert_eq!(
        assistant_content(&body),
        &json!([{"type": "tool_use", "id": "t1", "name": "lookup", "input": {"q": 1}}])
    );
    assert!(!body.to_string().contains("call-replay-secret"));
    assert!(!body.to_string().contains("other/format"));
}

#[test]
fn given_a_tool_call_with_an_item_reference_in_the_turn_in_progress_when_encoded_then_the_reference_is_not_sent()
 {
    let call = plain_call().with_item_ref(item_ref("call-item-secret"));
    let (body, parts) = body_with(Vec::new(), &call);
    assert!(matches!(&parts[0], RenderedPart::ToolCall { call, .. } if call.item_ref().is_some()));
    assert_eq!(
        assistant_content(&body),
        &json!([{"type": "tool_use", "id": "t1", "name": "lookup", "input": {"q": 1}}])
    );
    assert!(!body.to_string().contains("call-item-secret"));
}

#[test]
fn given_a_text_block_with_an_item_reference_in_the_turn_in_progress_when_encoded_then_the_reference_is_not_sent()
 {
    let text = ContentBlock::Text(
        TextBlock::new(block_id(), "referenced")
            .expect("a text")
            .with_item_ref(item_ref("text-item-secret")),
    );
    let (body, parts) = body_with(vec![text], &plain_call());
    assert!(matches!(
        &parts[0],
        RenderedPart::Block(ContentBlock::Text(_))
    ));
    assert_eq!(
        assistant_content(&body)[0],
        json!({"type": "text", "text": "referenced"})
    );
    assert!(!body.to_string().contains("text-item-secret"));
}

#[test]
fn given_a_text_block_with_replay_material_in_the_turn_in_progress_when_encoded_then_the_material_is_not_sent()
 {
    let text = ContentBlock::Text(
        TextBlock::new(block_id(), "sealed")
            .expect("a text")
            .with_replay(material("other/format", "text-replay-secret")),
    );
    let (body, parts) = body_with(vec![text], &plain_call());
    assert!(matches!(
        &parts[0],
        RenderedPart::Block(ContentBlock::Text(_))
    ));
    assert_eq!(
        assistant_content(&body)[0],
        json!({"type": "text", "text": "sealed"})
    );
    assert!(!body.to_string().contains("text-replay-secret"));
    assert!(!body.to_string().contains("other/format"));
}
