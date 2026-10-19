use br_llm_clients_core::ClientError;
use br_llm_messages::{Base64Data, Image, ImageMime, Text, WireMessage, WireUserBlock};
use serde_json::json;

use crate::test_requests::{
    assistant, assistant_text, call_block, input_of, refusal_of, result_block, text, user,
    user_text,
};

fn text_block(value: &str) -> WireUserBlock {
    WireUserBlock::Text { text: text(value) }
}

fn image_block(mime: ImageMime, data: &str) -> WireUserBlock {
    WireUserBlock::Image(Image {
        mime,
        data: Base64Data::new(data).expect("base64"),
    })
}

#[test]
fn given_a_user_text_when_encoded_then_it_is_one_message_item_with_an_input_text_part() {
    assert_eq!(
        input_of(vec![user_text("hello")]),
        [json!({
            "type": "message",
            "role": "user",
            "content": [{ "type": "input_text", "text": "hello" }]
        })]
    );
}

#[test]
fn given_text_and_image_blocks_when_encoded_then_they_are_parts_of_one_item_in_order() {
    let items = input_of(vec![user(vec![
        text_block("look"),
        image_block(ImageMime::Png, "iVBORw0K"),
        text_block("what is it"),
    ])]);
    assert_eq!(
        items,
        [json!({
            "type": "message",
            "role": "user",
            "content": [
                { "type": "input_text", "text": "look" },
                { "type": "input_image", "image_url": "data:image/png;base64,iVBORw0K" },
                { "type": "input_text", "text": "what is it" }
            ]
        })]
    );
}

#[test]
fn given_every_image_type_when_encoded_then_the_data_url_names_the_media_type_and_no_detail_is_sent()
 {
    for (mime, name) in [
        (ImageMime::Jpeg, "image/jpeg"),
        (ImageMime::Png, "image/png"),
        (ImageMime::Gif, "image/gif"),
        (ImageMime::Webp, "image/webp"),
    ] {
        let items = input_of(vec![user(vec![image_block(mime, "QUJD")])]);
        let part = &items[0]["content"][0];
        assert_eq!(part["image_url"], json!(format!("data:{name};base64,QUJD")));
        assert!(part.get("detail").is_none());
    }
}

#[test]
fn given_a_tool_result_when_encoded_then_it_is_a_function_call_output_with_the_call_id() {
    let messages = vec![
        user_text("go"),
        assistant(vec![call_block("call_1", "search", json!({}))]),
        user(vec![result_block("call_1", "search", &["found it"], false)]),
    ];
    let items = input_of(messages);
    assert_eq!(
        items[2],
        json!({ "type": "function_call_output", "call_id": "call_1", "output": "found it" })
    );
}

#[test]
fn given_a_result_with_several_texts_when_encoded_then_they_join_with_one_line_feed() {
    let messages = vec![
        user_text("go"),
        assistant(vec![call_block("c", "search", json!({}))]),
        user(vec![result_block(
            "c",
            "search",
            &["one", "two", "three"],
            false,
        )]),
    ];
    assert_eq!(input_of(messages)[2]["output"], json!("one\ntwo\nthree"));
}

#[test]
fn given_a_result_without_block_when_encoded_then_the_output_is_the_empty_text() {
    let messages = vec![
        user_text("go"),
        assistant(vec![call_block("c", "search", json!({}))]),
        user(vec![result_block("c", "search", &[], false)]),
    ];
    assert_eq!(input_of(messages)[2]["output"], json!(""));
}

#[test]
fn given_an_error_result_when_encoded_then_the_output_has_the_error_prefix() {
    let messages = vec![
        user_text("go"),
        assistant(vec![call_block("c", "search", json!({}))]),
        user(vec![result_block("c", "search", &["boom", "again"], true)]),
    ];
    assert_eq!(input_of(messages)[2]["output"], json!("Error: boom\nagain"));
}

#[test]
fn given_an_empty_error_result_when_encoded_then_the_output_is_the_bare_prefix() {
    let messages = vec![
        user_text("go"),
        assistant(vec![call_block("c", "search", json!({}))]),
        user(vec![result_block("c", "search", &[], true)]),
    ];
    assert_eq!(input_of(messages)[2]["output"], json!("Error: "));
}

#[test]
fn given_a_success_result_when_encoded_then_the_output_has_no_prefix() {
    let messages = vec![
        user_text("go"),
        assistant(vec![call_block("c", "search", json!({}))]),
        user(vec![result_block(
            "c",
            "search",
            &["Error: not really"],
            false,
        )]),
    ];
    assert_eq!(input_of(messages)[2]["output"], json!("Error: not really"));
}

#[test]
fn given_text_then_result_then_text_when_encoded_then_runs_and_results_keep_the_order_of_the_blocks()
 {
    let messages = vec![
        user_text("go"),
        assistant(vec![
            call_block("a", "search", json!({})),
            call_block("b", "search", json!({})),
        ]),
        user(vec![
            text_block("before"),
            result_block("a", "search", &["ra"], false),
            result_block("b", "search", &["rb"], false),
            text_block("middle"),
            image_block(ImageMime::Gif, "QUJD"),
            text_block("after"),
        ]),
    ];
    let items = input_of(messages);
    let kinds: Vec<&str> = items[3..]
        .iter()
        .map(|item| item["type"].as_str().expect("type"))
        .collect();
    assert_eq!(
        kinds,
        [
            "message",
            "function_call_output",
            "function_call_output",
            "message"
        ]
    );
    assert_eq!(items[3]["content"][0]["text"], json!("before"));
    assert_eq!(items[4]["call_id"], json!("a"));
    assert_eq!(items[5]["call_id"], json!("b"));
    assert_eq!(items[6]["content"].as_array().expect("parts").len(), 3);
}

#[test]
fn given_a_relay_when_encoded_then_it_is_a_user_message_with_one_input_text_per_text() {
    let relay = WireMessage::Relay {
        content: vec![
            Text::new("new mail from Ana").expect("text"),
            Text::new("another input").expect("text"),
        ],
    };
    let items = input_of(vec![user_text("hello"), assistant_text("ok"), relay]);
    assert_eq!(
        items[2],
        json!({
            "type": "message",
            "role": "user",
            "content": [
                { "type": "input_text", "text": "new mail from Ana" },
                { "type": "input_text", "text": "another input" }
            ]
        })
    );
}

#[test]
fn given_a_last_assistant_message_when_encoded_then_the_messages_are_invalid() {
    let refusal = refusal_of(vec![user_text("hello"), assistant_text("done")]);
    assert!(matches!(refusal, ClientError::InvalidMessages { .. }));
}

#[test]
fn given_a_last_assistant_message_of_only_a_call_when_encoded_then_the_messages_are_invalid() {
    let refusal = refusal_of(vec![
        user_text("hello"),
        assistant(vec![call_block("c", "search", json!({}))]),
    ]);
    assert!(matches!(refusal, ClientError::InvalidMessages { .. }));
}

#[test]
fn given_a_dialect_when_a_pairing_is_broken_then_encode_does_not_check_it() {
    let items = input_of(vec![
        user_text("hello"),
        assistant(vec![call_block("c", "search", json!({}))]),
        user_text("no result for it"),
    ]);
    assert_eq!(items.len(), 3);
}
