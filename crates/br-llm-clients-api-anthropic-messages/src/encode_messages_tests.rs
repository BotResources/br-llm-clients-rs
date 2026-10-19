use br_llm_messages::{AssistantBlock, ImageMime, WireUserBlock};
use serde_json::{Value, json};

use crate::test_fixtures::*;

fn messages_of(messages: Vec<br_llm_messages::WireMessage>) -> Value {
    body_of(&dialect(), &request(messages))["messages"].clone()
}

#[test]
fn given_user_and_assistant_messages_when_encoded_then_the_roles_are_kept_in_order() {
    let messages = messages_of(vec![user("a"), assistant_text("b"), user("c")]);
    assert_eq!(
        messages,
        json!([
            {"role": "user", "content": [{"type": "text", "text": "a"}]},
            {"role": "assistant", "content": [{"type": "text", "text": "b"}]},
            {"role": "user", "content": [{"type": "text", "text": "c"}]},
        ])
    );
}

#[test]
fn given_a_text_with_whitespace_when_encoded_then_it_is_sent_as_it_is() {
    let messages = messages_of(vec![user("  line one\n\n\tline two  ")]);
    assert_eq!(
        messages[0]["content"][0]["text"],
        json!("  line one\n\n\tline two  ")
    );
}

#[test]
fn given_each_image_type_when_encoded_then_it_is_a_base64_source_with_its_media_type() {
    for (mime, name) in [
        (ImageMime::Jpeg, "image/jpeg"),
        (ImageMime::Png, "image/png"),
        (ImageMime::Gif, "image/gif"),
        (ImageMime::Webp, "image/webp"),
    ] {
        let messages = messages_of(vec![user_blocks(vec![
            image(mime, "QUJD"),
            user_text_block("what"),
        ])]);
        assert_eq!(
            messages[0]["content"],
            json!([
                {"type": "image", "source": {"type": "base64", "media_type": name, "data": "QUJD"}},
                {"type": "text", "text": "what"},
            ])
        );
    }
}

#[test]
fn given_a_tool_result_when_encoded_then_it_holds_the_id_and_text_blocks_and_no_name() {
    let messages = messages_of(vec![
        user("go"),
        assistant(vec![tool_call("t1", "lookup", json!({}))]),
        user_blocks(vec![tool_result("t1", "lookup", &["one", " two "], false)]),
    ]);
    assert_eq!(
        messages[2]["content"],
        json!([{
            "type": "tool_result",
            "tool_use_id": "t1",
            "content": [{"type": "text", "text": "one"}, {"type": "text", "text": " two "}],
        }])
    );
    assert!(!messages[2].to_string().contains("lookup"));
}

#[test]
fn given_an_error_result_when_encoded_then_is_error_is_true_and_a_success_has_no_such_field() {
    let messages = messages_of(vec![
        user("go"),
        assistant(vec![
            tool_call("t1", "lookup", json!({})),
            tool_call("t2", "lookup", json!({})),
        ]),
        user_blocks(vec![
            tool_result("t1", "lookup", &["bad"], true),
            tool_result("t2", "lookup", &["fine"], false),
        ]),
    ]);
    let results = &messages[2]["content"];
    assert_eq!(results[0]["is_error"], json!(true));
    assert!(results[1].get("is_error").is_none());
}

#[test]
fn given_a_result_with_no_block_when_encoded_then_content_is_absent() {
    let messages = messages_of(vec![
        user("go"),
        assistant(vec![tool_call("t1", "lookup", json!({}))]),
        user_blocks(vec![tool_result("t1", "lookup", &[], false)]),
    ]);
    assert_eq!(
        messages[2]["content"],
        json!([{"type": "tool_result", "tool_use_id": "t1"}])
    );
}

#[test]
fn given_results_after_other_blocks_when_encoded_then_the_results_come_first_in_their_order() {
    let messages = messages_of(vec![
        user("go"),
        assistant(vec![
            tool_call("t1", "lookup", json!({})),
            tool_call("t2", "lookup", json!({})),
        ]),
        user_blocks(vec![
            user_text_block("note one"),
            tool_result("t1", "lookup", &["r1"], false),
            image(ImageMime::Png, "AAAA"),
            tool_result("t2", "lookup", &["r2"], false),
            user_text_block("note two"),
        ]),
    ]);
    let kinds: Vec<_> = messages[2]["content"]
        .as_array()
        .expect("array")
        .iter()
        .map(|block| block["type"].as_str().expect("type").to_owned())
        .collect();
    assert_eq!(
        kinds,
        ["tool_result", "tool_result", "text", "image", "text"]
    );
    assert_eq!(messages[2]["content"][0]["tool_use_id"], json!("t1"));
    assert_eq!(messages[2]["content"][1]["tool_use_id"], json!("t2"));
    assert_eq!(messages[2]["content"][2]["text"], json!("note one"));
    assert_eq!(messages[2]["content"][4]["text"], json!("note two"));
}

#[test]
fn given_a_relay_after_a_user_message_when_encoded_then_its_texts_end_that_message_after_the_results()
 {
    let messages = messages_of(vec![
        user("go"),
        assistant(vec![tool_call("t1", "lookup", json!({}))]),
        user_blocks(vec![tool_result("t1", "lookup", &["r1"], false)]),
        relay(&["first", "second"]),
    ]);
    assert_eq!(messages.as_array().expect("array").len(), 3);
    assert_eq!(
        messages[2]["content"],
        json!([
            {"type": "tool_result", "tool_use_id": "t1", "content": [{"type": "text", "text": "r1"}]},
            {"type": "text", "text": "first"},
            {"type": "text", "text": "second"},
        ])
    );
}

#[test]
fn given_a_relay_after_an_assistant_message_when_encoded_then_it_is_a_new_user_message() {
    let messages = messages_of(vec![user("a"), assistant_text("b"), relay(&["x", "y"])]);
    assert_eq!(
        messages,
        json!([
            {"role": "user", "content": [{"type": "text", "text": "a"}]},
            {"role": "assistant", "content": [{"type": "text", "text": "b"}]},
            {"role": "user", "content": [
                {"type": "text", "text": "x"},
                {"type": "text", "text": "y"},
            ]},
        ])
    );
}

#[test]
fn given_a_tool_call_when_encoded_then_it_is_a_tool_use_with_the_id_name_and_input_as_they_are() {
    let messages = messages_of(vec![
        user("go"),
        assistant(vec![tool_call(
            "brgen_x1",
            "lookup",
            json!({"q": [1, {"a": null}]}),
        )]),
        user_blocks(vec![tool_result("brgen_x1", "lookup", &["r"], false)]),
    ]);
    assert_eq!(
        messages[1]["content"],
        json!([{
            "type": "tool_use",
            "id": "brgen_x1",
            "name": "lookup",
            "input": {"q": [1, {"a": null}]},
        }])
    );
    assert_eq!(messages[2]["content"][0]["tool_use_id"], json!("brgen_x1"));
}

#[test]
fn given_an_id_the_api_would_refuse_when_encoded_then_it_is_sent_unchanged() {
    let messages = messages_of(vec![
        user("go"),
        assistant(vec![tool_call("a b.c", "lookup", json!({}))]),
        user_blocks(vec![tool_result("a b.c", "lookup", &["r"], false)]),
    ]);
    assert_eq!(messages[1]["content"][0]["id"], json!("a b.c"));
}

#[test]
fn given_a_tool_call_with_no_tool_declared_when_encoded_then_nothing_is_checked() {
    let request = request(vec![
        user("go"),
        assistant(vec![tool_call("t1", "ghost", json!({}))]),
        user_blocks(vec![tool_result("t1", "ghost", &["r"], false)]),
    ]);
    assert!(br_llm_clients_common_call::Dialect::encode(&dialect(), &request).is_ok());
}

#[test]
fn given_a_structured_block_when_encoded_then_it_is_a_text_block_with_its_compact_json() {
    let value = json!({"answer": {"nested": [1, 2.5, "x"]}, "ok": true});
    let messages = messages_of(vec![
        user("a"),
        assistant(vec![AssistantBlock::Structured {
            value: value.clone(),
        }]),
        user("b"),
    ]);
    let sent = messages[1]["content"][0]["text"].as_str().expect("text");
    assert_eq!(messages[1]["content"][0]["type"], json!("text"));
    assert_eq!(sent, value.to_string());
    assert!(!sent.contains(": ") && !sent.contains(", "));
    assert_eq!(serde_json::from_str::<Value>(sent).expect("json"), value);
}

#[test]
fn given_an_assistant_text_block_when_encoded_then_it_stays_a_text_block() {
    let messages = messages_of(vec![user("a"), assistant_text(" b "), user("c")]);
    assert_eq!(
        messages[1]["content"],
        json!([{"type": "text", "text": " b "}])
    );
}

#[test]
fn given_user_image_and_text_when_encoded_then_every_block_is_kept() {
    let messages = messages_of(vec![user_blocks(vec![
        WireUserBlock::Text { text: text("a") },
        image(ImageMime::Gif, "AAAA"),
    ])]);
    assert_eq!(messages[0]["content"].as_array().expect("array").len(), 2);
}
