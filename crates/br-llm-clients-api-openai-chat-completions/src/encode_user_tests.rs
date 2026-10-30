use br_llm_messages::{MediaType, PlainBlock, RuntimeToolCall, Turn};
use serde_json::{Value, json};

use crate::test_support::{dialect, messages_of, request_of, roles};
use crate::test_thread::{
    call, call_block, calling, earlier, human, plain_image, plain_text, result_of, results, texts,
    user,
};

fn encoded(turns: Vec<Turn>) -> Vec<Value> {
    messages_of(&dialect(), &request_of(turns))
}

fn lookups() -> [RuntimeToolCall; 2] {
    [
        call("c1", "lookup", json!({})),
        call("c2", "lookup", json!({})),
    ]
}

fn answered(first: Vec<PlainBlock>, second: Vec<PlainBlock>, is_error: bool) -> Vec<Turn> {
    let [one, two] = lookups();
    vec![
        user("go"),
        earlier(vec![
            calling(vec![call_block(&one), call_block(&two)]),
            results(vec![
                result_of(&one, first, is_error),
                result_of(&two, second, false),
            ]),
        ]),
    ]
}

#[test]
fn given_one_text_part_when_encoded_then_the_user_content_is_a_plain_string() {
    assert_eq!(
        encoded(vec![user("hi")]),
        [json!({"role": "user", "content": "hi"})]
    );
}

#[test]
fn given_two_text_parts_when_encoded_then_the_content_is_an_array_of_text_parts_in_order() {
    assert_eq!(
        encoded(vec![human(texts(&["one", "two"]))]),
        [json!({"role": "user", "content": [
            {"type": "text", "text": "one"},
            {"type": "text", "text": "two"}
        ]})]
    );
}

#[test]
fn given_an_image_and_a_text_when_encoded_then_the_parts_follow_the_order_of_the_rendering() {
    let turn = human(vec![
        plain_image(MediaType::Png, "aGVsbG8="),
        plain_text("what is this"),
    ]);
    assert_eq!(
        encoded(vec![turn]),
        [json!({"role": "user", "content": [
            {"type": "image_url", "image_url": {"url": "data:image/png;base64,aGVsbG8="}},
            {"type": "text", "text": "what is this"}
        ]})]
    );
}

#[test]
fn given_a_lone_image_when_encoded_then_the_content_is_still_an_array() {
    let turn = human(vec![plain_image(MediaType::Jpeg, "QUJD")]);
    assert_eq!(
        encoded(vec![turn]),
        [json!({"role": "user", "content": [
            {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,QUJD"}}
        ]})]
    );
}

#[test]
fn given_every_image_media_type_when_encoded_then_each_is_written_without_a_detail() {
    let media = [
        (MediaType::Jpeg, "image/jpeg"),
        (MediaType::Png, "image/png"),
        (MediaType::Gif, "image/gif"),
        (MediaType::Webp, "image/webp"),
    ];
    for (media_type, written) in media {
        let messages = encoded(vec![human(vec![plain_image(media_type, "AAAA")])]);
        let part = &messages[0]["content"][0];
        assert_eq!(
            part["image_url"]["url"],
            json!(format!("data:{written};base64,AAAA"))
        );
        assert_eq!(part["image_url"].as_object().expect("object").len(), 1);
        assert_eq!(part.as_object().expect("object").len(), 2);
    }
}

#[test]
fn given_a_result_with_one_text_when_encoded_then_the_tool_message_holds_a_string() {
    let messages = encoded(answered(texts(&["42"]), texts(&["x"]), false));
    assert_eq!(
        messages[2],
        json!({"role": "tool", "tool_call_id": "c1", "content": "42"})
    );
}

#[test]
fn given_a_result_with_several_texts_when_encoded_then_the_content_is_an_array_of_text_parts() {
    let messages = encoded(answered(texts(&["a", "b"]), texts(&["x"]), false));
    assert_eq!(
        messages[2]["content"],
        json!([{"type": "text", "text": "a"}, {"type": "text", "text": "b"}])
    );
}

#[test]
fn given_a_result_with_no_text_when_encoded_then_the_content_is_an_empty_string() {
    let messages = encoded(answered(Vec::new(), texts(&["x"]), false));
    assert_eq!(messages[2]["content"], json!(""));
}

#[test]
fn given_an_error_result_when_encoded_then_neither_the_flag_nor_the_tool_name_is_carried() {
    let failed = encoded(answered(texts(&["boom"]), texts(&["x"]), true));
    let fine = encoded(answered(texts(&["boom"]), texts(&["x"]), false));
    assert_eq!(
        failed[2],
        json!({"role": "tool", "tool_call_id": "c1", "content": "boom"})
    );
    assert_eq!(failed, fine);
}

#[test]
fn given_results_and_user_parts_in_one_message_when_encoded_then_the_tool_messages_come_first() {
    let mut turns = answered(texts(&["first"]), texts(&["second"]), false);
    turns.push(human(vec![
        plain_text("thanks"),
        plain_image(MediaType::Gif, "AAAA"),
    ]));
    let messages = encoded(turns);
    assert_eq!(
        messages[2..],
        [
            json!({"role": "tool", "tool_call_id": "c1", "content": "first"}),
            json!({"role": "tool", "tool_call_id": "c2", "content": "second"}),
            json!({"role": "user", "content": [
                {"type": "text", "text": "thanks"},
                {"type": "image_url", "image_url": {"url": "data:image/gif;base64,AAAA"}}
            ]}),
        ]
    );
}

#[test]
fn given_results_and_exactly_one_text_when_encoded_then_the_user_message_holds_a_string() {
    let mut turns = answered(texts(&["x"]), texts(&["y"]), false);
    turns.push(user("continue"));
    let messages = encoded(turns);
    assert_eq!(messages[4], json!({"role": "user", "content": "continue"}));
}

#[test]
fn given_only_results_when_encoded_then_no_user_message_follows_the_tool_messages() {
    let messages = encoded(answered(texts(&["x"]), texts(&["y"]), false));
    assert_eq!(roles(&messages), ["user", "assistant", "tool", "tool"]);
}

#[test]
fn given_a_result_holding_a_structured_child_when_encoded_then_it_is_its_compact_json_text() {
    let one = call("c1", "lookup", json!({}));
    let child = PlainBlock::StructuredContent(
        br_llm_messages::StructuredBlock::new(
            crate::test_thread::block_id(),
            br_llm_messages::OpaqueJson::parse(r#"{"a": 1}"#).expect("json"),
        )
        .expect("structured"),
    );
    let messages = encoded(vec![
        user("go"),
        earlier(vec![
            calling(vec![call_block(&one)]),
            results(vec![result_of(&one, vec![child], false)]),
        ]),
    ]);
    assert_eq!(messages[2]["content"], json!(r#"{"a":1}"#));
}
