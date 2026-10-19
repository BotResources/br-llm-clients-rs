use br_llm_messages::{ImageMime, WireMessage};
use serde_json::{Value, json};

use crate::test_support::{
    assistant, assistant_call, assistant_text, dialect, encoded_body, image, request_of, text,
    tool_result, user, user_text,
};
use br_llm_messages::WireUserBlock;

fn messages_of(list: Vec<WireMessage>) -> Value {
    encoded_body(&dialect(), &request_of(list))["messages"].clone()
}

fn text_block(value: &str) -> WireUserBlock {
    WireUserBlock::Text { text: text(value) }
}

fn conversation_with(results: Vec<WireUserBlock>) -> Vec<WireMessage> {
    let calls = ["c1", "c2", "c3"]
        .iter()
        .map(|id| assistant_call(id, "lookup", json!({})))
        .collect();
    vec![user_text("go"), assistant(calls), user(results)]
}

fn tail(messages: Value, skip: usize) -> Value {
    Value::Array(
        messages
            .as_array()
            .expect("array")
            .iter()
            .skip(skip)
            .cloned()
            .collect(),
    )
}

#[test]
fn given_one_text_block_when_encoded_then_the_user_content_is_a_plain_string() {
    assert_eq!(
        messages_of(vec![user_text("hi")]),
        json!([{"role": "user", "content": "hi"}])
    );
}

#[test]
fn given_two_text_blocks_when_encoded_then_the_content_is_an_array_of_text_parts_in_order() {
    let messages = messages_of(vec![user(vec![text_block("one"), text_block("two")])]);
    assert_eq!(
        messages,
        json!([{"role": "user", "content": [
            {"type": "text", "text": "one"},
            {"type": "text", "text": "two"}
        ]}])
    );
}

#[test]
fn given_a_text_and_an_image_when_encoded_then_the_parts_follow_the_block_order() {
    let messages = messages_of(vec![user(vec![
        image(ImageMime::Png, "aGVsbG8="),
        text_block("what is this"),
    ])]);
    assert_eq!(
        messages,
        json!([{"role": "user", "content": [
            {"type": "image_url", "image_url": {"url": "data:image/png;base64,aGVsbG8="}},
            {"type": "text", "text": "what is this"}
        ]}])
    );
}

#[test]
fn given_a_lone_image_when_encoded_then_the_content_is_still_an_array() {
    let messages = messages_of(vec![user(vec![image(ImageMime::Jpeg, "QUJD")])]);
    assert_eq!(
        messages,
        json!([{"role": "user", "content": [
            {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,QUJD"}}
        ]}])
    );
}

#[test]
fn given_every_image_media_type_when_encoded_then_each_is_written_without_a_detail() {
    let mimes = [
        (ImageMime::Jpeg, "image/jpeg"),
        (ImageMime::Png, "image/png"),
        (ImageMime::Gif, "image/gif"),
        (ImageMime::Webp, "image/webp"),
    ];
    for (mime, written) in mimes {
        let messages = messages_of(vec![user(vec![image(mime, "AAAA")])]);
        let part = &messages[0]["content"][0];
        assert_eq!(
            part["image_url"]["url"],
            json!(format!("data:{written};base64,AAAA"))
        );
        assert_eq!(part["image_url"].as_object().expect("object").len(), 1);
    }
}

#[test]
fn given_a_result_with_one_text_when_encoded_then_the_tool_message_holds_a_string() {
    let list = conversation_with(vec![tool_result("c1", "lookup", &["42"], false)]);
    let messages = messages_of(list);
    assert_eq!(
        tail(messages, 2),
        json!([{"role": "tool", "tool_call_id": "c1", "content": "42"}])
    );
}

#[test]
fn given_a_result_with_several_texts_when_encoded_then_the_content_is_an_array_of_text_parts() {
    let list = conversation_with(vec![tool_result("c1", "lookup", &["a", "b"], false)]);
    let messages = messages_of(list);
    assert_eq!(
        messages[2]["content"],
        json!([{"type": "text", "text": "a"}, {"type": "text", "text": "b"}])
    );
}

#[test]
fn given_a_result_with_no_text_when_encoded_then_the_content_is_an_empty_string() {
    let list = conversation_with(vec![tool_result("c1", "lookup", &[], false)]);
    assert_eq!(messages_of(list)[2]["content"], json!(""));
}

#[test]
fn given_an_error_result_when_encoded_then_neither_the_flag_nor_the_tool_name_is_carried() {
    let list = conversation_with(vec![tool_result("c1", "lookup", &["boom"], true)]);
    let messages = messages_of(list);
    assert_eq!(
        messages[2],
        json!({"role": "tool", "tool_call_id": "c1", "content": "boom"})
    );
    let plain = conversation_with(vec![tool_result("c1", "other_name", &["boom"], false)]);
    assert_eq!(messages_of(plain)[2], messages[2]);
}

#[test]
fn given_results_and_a_text_when_encoded_then_the_tool_messages_come_first_in_block_order() {
    let list = conversation_with(vec![
        text_block("thanks"),
        tool_result("c2", "lookup", &["second"], false),
        image(ImageMime::Gif, "AAAA"),
        tool_result("c1", "lookup", &["first"], false),
    ]);
    let messages = messages_of(list);
    assert_eq!(
        tail(messages, 2),
        json!([
            {"role": "tool", "tool_call_id": "c2", "content": "second"},
            {"role": "tool", "tool_call_id": "c1", "content": "first"},
            {"role": "user", "content": [
                {"type": "text", "text": "thanks"},
                {"type": "image_url", "image_url": {"url": "data:image/gif;base64,AAAA"}}
            ]}
        ])
    );
}

#[test]
fn given_results_and_exactly_one_text_when_encoded_then_the_user_message_holds_a_string() {
    let list = conversation_with(vec![
        tool_result("c1", "lookup", &["x"], false),
        text_block("continue"),
    ]);
    let messages = messages_of(list);
    assert_eq!(messages[3], json!({"role": "user", "content": "continue"}));
}

#[test]
fn given_only_results_when_encoded_then_no_user_message_follows_the_tool_messages() {
    let list = conversation_with(vec![
        tool_result("c1", "lookup", &["x"], false),
        tool_result("c2", "lookup", &["y"], false),
    ]);
    let messages = messages_of(list);
    let roles: Vec<&str> = messages
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|message| message["role"].as_str())
        .collect();
    assert_eq!(roles, ["user", "assistant", "tool", "tool"]);
}

#[test]
fn given_a_relay_when_encoded_then_it_is_one_user_message_with_an_array_even_for_one_text() {
    let list = vec![
        user_text("go"),
        assistant(vec![assistant_text("working")]),
        user(vec![text_block("more")]),
        WireMessage::Relay {
            content: vec![text("note")],
        },
    ];
    let messages = messages_of(list);
    assert_eq!(
        messages[3],
        json!({"role": "user", "content": [{"type": "text", "text": "note"}]})
    );
}

#[test]
fn given_a_relay_with_several_texts_when_encoded_then_each_text_is_a_part_in_order() {
    let list = vec![
        user_text("go"),
        WireMessage::Relay {
            content: vec![text("a"), text("b")],
        },
    ];
    assert_eq!(
        messages_of(list)[1]["content"],
        json!([{"type": "text", "text": "a"}, {"type": "text", "text": "b"}])
    );
}

#[test]
fn given_results_then_a_relay_when_encoded_then_the_relay_texts_arrive_after_the_tool_results() {
    let mut list = conversation_with(vec![tool_result("c1", "lookup", &["x"], false)]);
    list.push(WireMessage::Relay {
        content: vec![text("note")],
    });
    let messages = messages_of(list);
    let roles: Vec<&str> = messages
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|message| message["role"].as_str())
        .collect();
    assert_eq!(roles, ["user", "assistant", "tool", "user"]);
}
