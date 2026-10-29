use br_llm_messages::{ContentBlock, ReplayMaterial, ReplayPayload};
use serde_json::{Value, json};

use crate::envelope::REPLAY_FORMAT;
use crate::test_history::{
    call, earlier, envelope_replay, envelope_text, json as opaque, notice, plain_text, replay,
    reply, response, result_of, results, say, text, thinking,
};
use crate::test_requests::input_of;

fn reasoning(id: &str, encrypted: &str) -> Value {
    json!({ "type": "reasoning", "id": id, "summary": [], "encrypted_content": encrypted })
}

fn reasoning_ids(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter(|item| item["type"] == json!("reasoning"))
        .map(|item| item["id"].as_str().expect("id").to_owned())
        .collect()
}

fn with_thinking(block: ContentBlock) -> Vec<Value> {
    let found = call(Some("c"), "search", "{}");
    input_of(vec![
        say("go"),
        reply(vec![
            response(vec![block, ContentBlock::RuntimeToolCall(found.clone())]),
            results(vec![result_of(&found, vec![plain_text("r")], false)]),
        ]),
    ])
}

#[test]
fn given_a_thinking_block_of_the_reply_turn_with_an_envelope_when_encoded_then_a_reasoning_item_is_sent()
 {
    let items = with_thinking(thinking("t", Some(envelope_replay("rs_1", "ENC"))));
    assert_eq!(items[1], reasoning("rs_1", "ENC"));
}

#[test]
fn given_the_text_of_a_thinking_block_when_encoded_then_it_is_never_sent() {
    let items = with_thinking(thinking(
        "SECRET THINKING TEXT",
        Some(envelope_replay("rs_1", "ENC")),
    ));
    let body = serde_json::to_string(&items).expect("json");
    assert!(!body.contains("SECRET THINKING TEXT"));
}

#[test]
fn given_a_thinking_block_of_an_earlier_turn_when_encoded_then_it_is_dropped_without_error() {
    let items = input_of(vec![
        say("first question"),
        earlier(vec![response(vec![
            thinking("old", Some(envelope_replay("rs_old", "OLD"))),
            text("first answer"),
        ])]),
        say("second question"),
    ]);
    assert_eq!(items.len(), 3);
    assert!(reasoning_ids(&items).is_empty());
}

#[test]
fn given_two_responses_in_the_reply_turn_when_encoded_then_both_thinking_blocks_are_sent() {
    let first = call(Some("c1"), "search", "{}");
    let second = call(Some("c2"), "search", "{}");
    let items = input_of(vec![
        say("go"),
        reply(vec![
            response(vec![
                thinking("a", Some(envelope_replay("rs_1", "E1"))),
                ContentBlock::RuntimeToolCall(first.clone()),
            ]),
            results(vec![result_of(&first, Vec::new(), false)]),
            response(vec![
                thinking("b", Some(envelope_replay("rs_2", "E2"))),
                ContentBlock::RuntimeToolCall(second.clone()),
            ]),
            results(vec![result_of(&second, Vec::new(), false)]),
        ]),
    ]);
    assert_eq!(reasoning_ids(&items), ["rs_1", "rs_2"]);
}

#[test]
fn given_a_thinking_block_without_replay_material_when_encoded_then_it_is_dropped_without_error() {
    let items = with_thinking(thinking("plain", None));
    assert!(reasoning_ids(&items).is_empty());
    assert_eq!(items.len(), 3);
}

#[test]
fn given_replay_material_of_another_format_when_encoded_then_the_block_is_dropped_without_error() {
    let material = replay(
        "br-llm-clients-api-anthropic-messages/signature",
        ReplayPayload::text(envelope_text("rs_1", "ENC")),
    );
    let items = with_thinking(thinking("t", Some(material)));
    assert!(reasoning_ids(&items).is_empty());
    assert_eq!(items.len(), 3);
}

#[test]
fn given_replay_material_of_this_format_that_is_not_an_envelope_when_encoded_then_the_block_is_dropped_without_error()
 {
    let texts = [
        "not json",
        "[]",
        "\"rs_1\"",
        "{}",
        "{\"id\":\"rs_1\"}",
        "{\"encrypted_content\":\"E\"}",
        "{\"id\":\"\",\"encrypted_content\":\"E\"}",
        "{\"id\":\"rs_1\",\"encrypted_content\":\"\"}",
        "{\"id\":1,\"encrypted_content\":\"E\"}",
        "{\"id\":\"rs_1\",\"encrypted_content\":null}",
        "{\"id\":\"rs_1\",\"encrypted_content\":\"E\",\"extra\":1}",
        "{\"id\":\"rs_1\",\"other\":\"E\"}",
    ];
    for value in texts {
        let material = replay(REPLAY_FORMAT, ReplayPayload::text(value));
        let items = with_thinking(thinking("t", Some(material)));
        assert!(reasoning_ids(&items).is_empty(), "{value}");
        assert_eq!(items.len(), 3, "{value}");
    }
}

#[test]
fn given_replay_material_of_this_format_held_as_json_when_encoded_then_the_block_is_dropped_without_error()
 {
    let payload = ReplayPayload::json(opaque(&envelope_text("rs_1", "ENC"))).expect("payload");
    let material: ReplayMaterial = replay(REPLAY_FORMAT, payload);
    let items = with_thinking(thinking("t", Some(material)));
    assert!(reasoning_ids(&items).is_empty());
}

#[test]
fn given_an_envelope_with_the_fields_in_another_order_when_encoded_then_it_is_still_sent() {
    let material = replay(
        REPLAY_FORMAT,
        ReplayPayload::text("{\"encrypted_content\":\"E\",\"id\":\"rs_9\"}"),
    );
    let items = with_thinking(thinking("t", Some(material)));
    assert_eq!(items[1], reasoning("rs_9", "E"));
}

#[test]
fn given_a_thinking_block_that_drops_out_beside_a_text_when_encoded_then_the_text_stays() {
    let items = input_of(vec![
        say("go"),
        reply(vec![
            response(vec![thinking("t", None), text("answer")]),
            notice("next"),
        ]),
    ]);
    assert_eq!(
        items[1],
        json!({ "type": "message", "role": "assistant", "content": "answer" })
    );
    assert_eq!(items.len(), 3);
}
