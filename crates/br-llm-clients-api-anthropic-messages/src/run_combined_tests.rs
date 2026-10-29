use br_llm_clients_core::ClientError;
use br_llm_messages::ResponseEnding;

use crate::test_call::*;
use crate::test_streams::BASIC;
use crate::test_streams_constructed::*;

fn message_start() -> String {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","model":"claude-test-model","usage":{"input_tokens":10,"output_tokens":2}}}

"#
    .to_owned()
}

fn frame(event: &str, data: &str) -> String {
    format!("event: {event}\ndata: {data}\n\n")
}

fn finish(stop_reason: Option<&str>) -> String {
    let reason = stop_reason.map_or("null".to_owned(), |reason| format!("\"{reason}\""));
    frame(
        "message_delta",
        &format!(
            r#"{{"type":"message_delta","delta":{{"stop_reason":{reason}}},"usage":{{"output_tokens":9}}}}"#
        ),
    ) + &frame("message_stop", r#"{"type":"message_stop"}"#)
}

fn text_block(text: &str, closed: bool) -> String {
    let text = serde_json::to_string(text).expect("json");
    let mut stream = frame(
        "content_block_start",
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
    );
    stream += &frame(
        "content_block_delta",
        &format!(
            r#"{{"type":"content_block_delta","index":0,"delta":{{"type":"text_delta","text":{text}}}}}"#
        ),
    );
    if closed {
        stream += &frame(
            "content_block_stop",
            r#"{"type":"content_block_stop","index":0}"#,
        );
    }
    stream
}

fn tool_block(partial: Option<&str>, closed: bool) -> String {
    let mut stream = frame(
        "content_block_start",
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"ping","input":{}}}"#,
    );
    if let Some(partial) = partial {
        let partial = serde_json::to_string(partial).expect("json");
        stream += &frame(
            "content_block_delta",
            &format!(
                r#"{{"type":"content_block_delta","index":0,"delta":{{"type":"input_json_delta","partial_json":{partial}}}}}"#
            ),
        );
    }
    if closed {
        stream += &frame(
            "content_block_stop",
            r#"{"type":"content_block_stop","index":0}"#,
        );
    }
    stream
}

fn malformed(error: &ClientError) -> bool {
    matches!(error, ClientError::MalformedResponse { .. })
}

#[tokio::test]
async fn given_a_held_tool_call_when_the_stop_reason_is_not_tool_use_then_common_call_judges_it_incomplete()
 {
    let with = |reason: &str| {
        EMPTY_TOOL_CALL.replace(
            r#""stop_reason":"tool_use""#,
            &format!(r#""stop_reason":{reason}"#),
        )
    };
    let refusal = call_stream(&hello(), &with("\"refusal\"")).await;
    assert_eq!(refusal.error(), &ClientError::ContentBlocked);
    for reason in [
        "\"end_turn\"",
        "\"pause_turn\"",
        "\"stop_sequence\"",
        "\"model_context_window_exceeded\"",
    ] {
        let called = call_stream(&hello(), &with(reason)).await;
        assert!(malformed(called.error()), "{reason}: {:?}", called.error());
    }
    let none = call_stream(&hello(), &with("null")).await;
    assert_eq!(none.error(), &ClientError::StreamTruncated);
    let max_tokens = call_stream(&hello(), &with("\"max_tokens\"")).await;
    assert_eq!(
        max_tokens.error(),
        &ClientError::ReplyUnusable {
            ending: ResponseEnding::output_exhaustion()
        }
    );
}

#[tokio::test]
async fn given_a_held_tool_call_under_tool_use_when_run_then_the_call_has_the_empty_object() {
    let called = call_stream(&hello(), EMPTY_TOOL_CALL).await;
    assert_eq!(crate::test_response::tool_calls(called.response()).len(), 1);
}

#[tokio::test]
async fn given_a_tool_call_with_cut_arguments_when_the_stop_reason_is_tool_use_then_the_reply_is_malformed()
 {
    for closed in [true, false] {
        let stream =
            message_start() + &tool_block(Some("{\"a\":"), closed) + &finish(Some("tool_use"));
        let called = call_stream(&hello(), &stream).await;
        assert!(
            malformed(called.error()),
            "closed={closed}: {:?}",
            called.error()
        );
    }
}

#[tokio::test]
async fn given_a_tool_call_with_complete_arguments_left_open_when_message_stop_arrives_then_it_is_judged_incomplete()
 {
    let stream =
        message_start() + &tool_block(Some("{\"a\":1}"), false) + &finish(Some("tool_use"));
    let called = call_stream(&hello(), &stream).await;
    assert!(malformed(called.error()), "{:?}", called.error());
}

#[tokio::test]
async fn given_a_text_block_left_open_when_message_stop_arrives_under_end_turn_then_the_step_holds_the_text()
 {
    let stream = message_start() + &text_block("Hello there", false) + &finish(Some("end_turn"));
    let called = call_stream(&hello(), &stream).await;
    let texts: Vec<_> = crate::test_response::texts(called.response());
    assert_eq!(texts, ["Hello there"]);
    assert_eq!(called.response().ending(), &ResponseEnding::finished());
}

#[tokio::test]
async fn given_a_basic_stream_without_its_content_block_stop_when_run_then_the_step_is_the_same() {
    let without_stop = BASIC.replace(
        "event: content_block_stop\ndata: {\"type\": \"content_block_stop\", \"index\": 0}\n\n",
        "",
    );
    assert_ne!(without_stop, BASIC);
    let called = call_stream(&hello(), &without_stop).await;
    let texts: Vec<_> = crate::test_response::texts(called.response());
    assert_eq!(texts, ["Hello!"]);
}

#[tokio::test]
async fn given_the_structured_mode_and_a_refusal_when_run_then_the_text_decides_the_error() {
    let prose = message_start() + &text_block("I can't help", true) + &finish(Some("refusal"));
    let called = call_with(plain_dialect(), &structured_hello(), move |_| {
        crate::test_server::http("200 OK", &[], prose.as_bytes())
    })
    .await;
    assert!(malformed(called.error()), "{:?}", called.error());

    let partial = message_start() + &text_block("{\"val", true) + &finish(Some("refusal"));
    let called = call_with(plain_dialect(), &structured_hello(), move |_| {
        crate::test_server::http("200 OK", &[], partial.as_bytes())
    })
    .await;
    assert_eq!(called.error(), &ClientError::ContentBlocked);

    let none = REFUSAL.to_owned();
    let called = call_with(plain_dialect(), &structured_hello(), move |_| {
        crate::test_server::http("200 OK", &[], none.as_bytes())
    })
    .await;
    assert_eq!(called.error(), &ClientError::ContentBlocked);
}

#[tokio::test]
async fn given_the_plain_mode_and_a_refusal_with_prose_when_run_then_the_step_keeps_the_prose_and_the_refusal()
 {
    let prose = message_start() + &text_block("I can't help", true) + &finish(Some("refusal"));
    let called = call_stream(&hello(), &prose).await;
    assert_eq!(called.response().ending(), &ResponseEnding::refusal());
    assert_eq!(crate::test_response::texts(called.response()).len(), 1);
}

#[tokio::test]
async fn given_the_structured_mode_and_a_blank_text_with_a_refusal_when_run_then_the_content_is_blocked()
 {
    let blank = message_start() + &text_block("   ", true) + &finish(Some("refusal"));
    let called = call_with(plain_dialect(), &structured_hello(), move |_| {
        crate::test_server::http("200 OK", &[], blank.as_bytes())
    })
    .await;
    assert_eq!(called.error(), &ClientError::ContentBlocked);
}
