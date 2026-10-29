use br_llm_clients_core::ReasoningLevel;
use br_llm_messages::{ContentBlock, MediaType};
use serde_json::{Value, json};

use crate::test_history::{
    call, human, plain_image, plain_text, reply, response, result_of, results, say,
};
use crate::test_requests::{dialect, hello, input_of, request};

fn input_text(text: &str) -> Value {
    json!({ "type": "input_text", "text": text })
}

fn user_item(parts: Vec<Value>) -> Value {
    json!({ "type": "message", "role": "user", "content": parts })
}

fn answered(result_content: Vec<br_llm_messages::PlainBlock>, is_error: bool) -> Vec<Value> {
    let call = call(Some("call_1"), "search", "{}");
    input_of(vec![
        say("go"),
        reply(vec![
            response(vec![ContentBlock::RuntimeToolCall(call.clone())]),
            results(vec![result_of(&call, result_content, is_error)]),
        ]),
    ])
}

fn output_of(items: &[Value]) -> Value {
    items
        .iter()
        .find(|item| item["type"] == json!("function_call_output"))
        .expect("a function call output")["output"]
        .clone()
}

#[test]
fn given_a_user_text_when_encoded_then_it_is_one_message_item_with_an_input_text_part() {
    assert_eq!(
        input_of(vec![say("hello")]),
        vec![user_item(vec![input_text("hello")])]
    );
}

#[test]
fn given_text_and_image_parts_when_encoded_then_they_are_parts_of_one_item_in_order() {
    let turn = human(vec![
        plain_text("look"),
        plain_image(MediaType::Png, "iVBORw0K"),
        plain_text("and this"),
    ]);
    assert_eq!(
        input_of(vec![turn]),
        vec![user_item(vec![
            input_text("look"),
            json!({ "type": "input_image", "image_url": "data:image/png;base64,iVBORw0K" }),
            input_text("and this"),
        ])]
    );
}

#[test]
fn given_every_image_type_when_encoded_then_the_data_url_names_the_media_type_and_no_detail_is_sent()
 {
    for media_type in [
        MediaType::Jpeg,
        MediaType::Png,
        MediaType::Gif,
        MediaType::Webp,
    ] {
        let items = input_of(vec![human(vec![plain_image(media_type, "QUJD")])]);
        let part = &items[0]["content"][0];
        assert_eq!(
            part,
            &json!({
                "type": "input_image",
                "image_url": format!("data:{};base64,QUJD", media_type.as_str())
            })
        );
        assert!(part.get("detail").is_none());
    }
}

#[test]
fn given_a_tool_result_when_encoded_then_it_is_a_function_call_output_with_the_wire_id() {
    let items = answered(vec![plain_text("18C")], false);
    assert_eq!(
        items[2],
        json!({ "type": "function_call_output", "call_id": "call_1", "output": "18C" })
    );
}

#[test]
fn given_a_result_with_several_texts_when_encoded_then_they_join_with_one_line_feed() {
    let items = answered(
        vec![plain_text("a"), plain_text("b"), plain_text("c")],
        false,
    );
    assert_eq!(output_of(&items), json!("a\nb\nc"));
}

#[test]
fn given_a_result_without_part_when_encoded_then_the_output_is_the_empty_text() {
    assert_eq!(output_of(&answered(Vec::new(), false)), json!(""));
}

#[test]
fn given_an_error_result_when_encoded_then_the_output_has_the_error_prefix() {
    let items = answered(vec![plain_text("boom"), plain_text("again")], true);
    assert_eq!(output_of(&items), json!("Error: boom\nagain"));
}

#[test]
fn given_an_empty_error_result_when_encoded_then_the_output_is_the_bare_prefix() {
    assert_eq!(output_of(&answered(Vec::new(), true)), json!("Error: "));
}

#[test]
fn given_a_success_result_when_encoded_then_the_output_has_no_prefix() {
    assert_eq!(
        output_of(&answered(vec![plain_text("Error: is my text")], false)),
        json!("Error: is my text")
    );
}

#[test]
fn given_a_result_with_an_image_when_encoded_then_the_output_is_an_array_of_parts_in_order() {
    let items = answered(
        vec![
            plain_text("chart"),
            plain_image(MediaType::Jpeg, "SkZJRg=="),
            plain_text("done"),
        ],
        false,
    );
    assert_eq!(
        output_of(&items),
        json!([
            { "type": "input_text", "text": "chart" },
            { "type": "input_image", "image_url": "data:image/jpeg;base64,SkZJRg==" },
            { "type": "input_text", "text": "done" }
        ])
    );
}

#[test]
fn given_an_error_result_with_an_image_when_encoded_then_the_array_starts_with_the_error_prefix_part()
 {
    let items = answered(vec![plain_image(MediaType::Png, "UE5H")], true);
    assert_eq!(
        output_of(&items),
        json!([
            { "type": "input_text", "text": "Error: " },
            { "type": "input_image", "image_url": "data:image/png;base64,UE5H" }
        ])
    );
}

#[test]
fn given_a_result_and_a_text_in_one_user_message_when_encoded_then_items_keep_the_order_of_the_parts()
 {
    let first = call(Some("call_a"), "search", "{}");
    let second = call(Some("call_b"), "search", "{}");
    let items = input_of(vec![
        say("go"),
        reply(vec![
            response(vec![
                ContentBlock::RuntimeToolCall(first.clone()),
                ContentBlock::RuntimeToolCall(second.clone()),
            ]),
            results(vec![
                result_of(&first, vec![plain_text("r1")], false),
                result_of(&second, vec![plain_text("r2")], false),
            ]),
            crate::test_history::notice("note"),
        ]),
    ]);
    let kinds: Vec<(&str, &str)> = items
        .iter()
        .map(|item| {
            (
                item["type"].as_str().expect("type"),
                item.get("call_id")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("role").and_then(Value::as_str))
                    .expect("call id or role"),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ("message", "user"),
            ("function_call", "call_a"),
            ("function_call", "call_b"),
            ("function_call_output", "call_a"),
            ("function_call_output", "call_b"),
            ("message", "user"),
        ]
    );
    assert_eq!(
        items[5]["content"],
        json!([input_text("<notice>note</notice>")])
    );
}

#[test]
fn given_a_request_and_the_rendering_of_another_when_encoded_then_the_input_is_the_given_rendering()
{
    let other = request(vec![say("from the rendering")]);
    let encoded = br_llm_clients_common_call::Dialect::encode(
        &dialect(ReasoningLevel::Off),
        &hello(),
        &crate::test_requests::rendered(&other),
    )
    .expect("encoded");
    let body: Value = serde_json::from_slice(encoded.body()).expect("json");
    assert_eq!(
        body["input"],
        json!([user_item(vec![input_text("from the rendering")])])
    );
}

#[test]
fn given_an_unresolved_call_of_a_closed_turn_when_encoded_then_the_result_built_by_the_core_is_an_error_output()
 {
    let found = call(Some("call_lost"), "search", "{}");
    let items = input_of(vec![
        say("go"),
        crate::test_history::earlier(vec![
            response(vec![ContentBlock::RuntimeToolCall(found)]),
            crate::test_history::closed(),
        ]),
        say("again"),
    ]);
    assert_eq!(
        items[2],
        json!({
            "type": "function_call_output",
            "call_id": "call_lost",
            "output": "Error: No result was recorded for this tool call because its turn ended before a result came."
        })
    );
    assert_eq!(
        items[3],
        user_item(vec![
            input_text("<notice>Your previous turn was interrupted.</notice>"),
            input_text("again"),
        ])
    );
}
