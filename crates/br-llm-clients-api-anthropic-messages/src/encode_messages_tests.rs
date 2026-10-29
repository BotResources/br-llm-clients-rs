use br_llm_messages::{MediaType, Turn};
use serde_json::{Value, json};

use crate::test_fixtures::*;
use crate::test_thread::*;

fn messages_of(turns: Vec<Turn>) -> Value {
    body_of(&dialect(), &request(turns))["messages"].clone()
}

pub(crate) fn one_call(id: &str, name: &str, arguments: Value, content: &[&str]) -> Vec<Turn> {
    let call = call(id, name, arguments);
    vec![
        user("go"),
        in_progress(vec![
            calling(vec![call_block(&call)]),
            results(vec![result_of(&call, texts(content), false)]),
        ]),
    ]
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
    for (media_type, name) in [
        (MediaType::Jpeg, "image/jpeg"),
        (MediaType::Png, "image/png"),
        (MediaType::Gif, "image/gif"),
        (MediaType::Webp, "image/webp"),
    ] {
        let messages = messages_of(vec![human_with(vec![
            plain_image(media_type, "QUJD"),
            plain_text("what"),
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
fn given_a_tool_result_when_encoded_then_it_holds_the_wire_id_and_text_blocks_and_no_name() {
    let messages = messages_of(one_call("t1", "lookup", json!({}), &["one", " two "]));
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
fn given_a_tool_result_with_an_image_when_encoded_then_its_content_holds_the_image_block_in_order()
{
    let call = call("t1", "snap", json!({}));
    let content = vec![plain_text("before"), plain_image(MediaType::Png, "AAAA")];
    let messages = messages_of(vec![
        user("go"),
        in_progress(vec![
            calling(vec![call_block(&call)]),
            results(vec![result_of(&call, content, false)]),
        ]),
    ]);
    assert_eq!(
        messages[2]["content"][0]["content"],
        json!([
            {"type": "text", "text": "before"},
            {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "AAAA"}},
        ])
    );
}

#[test]
fn given_an_error_result_when_encoded_then_is_error_is_true_and_a_success_has_no_such_field() {
    let first = call("t1", "lookup", json!({}));
    let second = call("t2", "lookup", json!({}));
    let messages = messages_of(vec![
        user("go"),
        in_progress(vec![
            calling(vec![call_block(&first), call_block(&second)]),
            results(vec![
                result_of(&first, texts(&["bad"]), true),
                result_of(&second, texts(&["fine"]), false),
            ]),
        ]),
    ]);
    let results = &messages[2]["content"];
    assert_eq!(results[0]["is_error"], json!(true));
    assert!(results[1].get("is_error").is_none());
}

#[test]
fn given_a_result_with_no_block_when_encoded_then_content_is_absent() {
    let messages = messages_of(one_call("t1", "lookup", json!({}), &[]));
    assert_eq!(
        messages[2]["content"],
        json!([{"type": "tool_result", "tool_use_id": "t1"}])
    );
}

#[test]
fn given_results_and_a_later_message_when_encoded_then_the_rendering_order_is_kept() {
    let first = call("t1", "lookup", json!({}));
    let second = call("t2", "lookup", json!({}));
    let messages = messages_of(vec![
        user("go"),
        in_progress(vec![
            calling(vec![call_block(&first), call_block(&second)]),
            results(vec![
                result_of(&second, texts(&["r2"]), false),
                result_of(&first, texts(&["r1"]), false),
            ]),
        ]),
        user("note"),
    ]);
    let kinds: Vec<_> = messages[2]["content"]
        .as_array()
        .expect("array")
        .iter()
        .map(|block| block["type"].as_str().expect("type").to_owned())
        .collect();
    assert_eq!(kinds, ["tool_result", "tool_result", "text"]);
    assert_eq!(messages[2]["content"][0]["tool_use_id"], json!("t1"));
    assert_eq!(messages[2]["content"][1]["tool_use_id"], json!("t2"));
    assert_eq!(messages[2]["content"][2]["text"], json!("note"));
}

#[test]
fn given_a_tool_call_when_encoded_then_it_is_a_tool_use_with_the_wire_id_name_and_input() {
    let messages = messages_of(one_call(
        "brgen_x1",
        "lookup",
        json!({"q": [1, {"a": null}]}),
        &["r"],
    ));
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
    let messages = messages_of(one_call("a b.c", "lookup", json!({}), &["r"]));
    assert_eq!(messages[1]["content"][0]["id"], json!("a b.c"));
    assert_eq!(messages[2]["content"][0]["tool_use_id"], json!("a b.c"));
}

#[test]
fn given_a_tool_call_with_no_tool_declared_when_encoded_then_nothing_is_checked() {
    let request = request(one_call("t1", "ghost", json!({}), &["r"]));
    assert!(encode(&dialect(), &request).is_ok());
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
fn given_user_text_and_image_when_encoded_then_every_block_is_kept_in_order() {
    let messages = messages_of(vec![human_with(vec![
        plain_text("a"),
        plain_image(MediaType::Gif, "AAAA"),
    ])]);
    assert_eq!(messages[0]["content"][0]["type"], json!("text"));
    assert_eq!(messages[0]["content"][1]["type"], json!("image"));
    assert_eq!(messages[0]["content"].as_array().expect("array").len(), 2);
}
