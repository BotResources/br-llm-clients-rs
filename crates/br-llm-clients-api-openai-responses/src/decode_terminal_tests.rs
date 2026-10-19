use br_llm_clients_common_call::{ErrorInfo, ProviderEvent};
use br_llm_messages::StopReason;
use serde_json::json;

use crate::test_decoder::{feed, feed_err, flat, model, plain_decoder, stop, usage};
use crate::test_events::{
    call_added, call_done, completed, error_event, failed, incomplete, message_added, usage_object,
};

fn error_of(events: &[ProviderEvent]) -> ErrorInfo {
    match events.last() {
        Some(ProviderEvent::Error(info)) => info.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn given_a_completed_response_when_finished_then_usage_model_stop_complete_come_in_this_order() {
    let mut decoder = plain_decoder();
    assert_eq!(
        feed(&mut decoder, &completed("gpt-5.1-x", usage_object(10, 20))),
        vec![
            usage(10, 20),
            model("gpt-5.1-x"),
            stop(StopReason::EndTurn),
            ProviderEvent::Complete
        ]
    );
}

#[test]
fn given_a_completed_response_without_usage_when_finished_then_there_is_no_usage_event() {
    let mut decoder = plain_decoder();
    assert_eq!(
        feed(&mut decoder, &completed("m", serde_json::Value::Null)),
        vec![
            model("m"),
            stop(StopReason::EndTurn),
            ProviderEvent::Complete
        ]
    );
}

#[test]
fn given_a_completed_response_with_an_empty_model_when_finished_then_there_is_no_model_event() {
    let mut decoder = plain_decoder();
    assert_eq!(
        feed(&mut decoder, &completed("", usage_object(1, 2))),
        vec![
            usage(1, 2),
            stop(StopReason::EndTurn),
            ProviderEvent::Complete
        ]
    );
}

#[test]
fn given_the_incomplete_reasons_when_finished_then_the_stop_follows_the_table() {
    let table = [
        (Some("max_output_tokens"), StopReason::MaxTokens),
        (Some("content_filter"), StopReason::ContentFilter),
        (
            Some("steered"),
            StopReason::Other {
                reason: "steered".to_owned(),
            },
        ),
        (
            Some(""),
            StopReason::Other {
                reason: String::new(),
            },
        ),
        (
            None,
            StopReason::Other {
                reason: "incomplete".to_owned(),
            },
        ),
    ];
    for (reason, expected) in table {
        let mut decoder = plain_decoder();
        let events = feed(&mut decoder, &incomplete(reason, usage_object(1, 1)));
        assert!(events.contains(&stop(expected)), "{reason:?}");
        assert_eq!(events.last(), Some(&ProviderEvent::Complete));
    }
}

#[test]
fn given_an_incomplete_response_with_details_without_reason_when_finished_then_the_stop_is_other_incomplete()
 {
    let mut decoder = plain_decoder();
    let mut event = incomplete(None, usage_object(1, 1));
    event["response"]["incomplete_details"] = json!({});
    let events = feed(&mut decoder, &event);
    assert!(events.contains(&stop(StopReason::Other {
        reason: "incomplete".to_owned()
    })));
}

#[test]
fn given_an_incomplete_response_when_finished_then_the_order_is_usage_model_stop_complete() {
    let mut decoder = plain_decoder();
    let events = feed(
        &mut decoder,
        &incomplete(Some("max_output_tokens"), usage_object(4, 5)),
    );
    assert_eq!(
        events,
        vec![
            usage(4, 5),
            model("gpt-5.1-2026-01-01"),
            stop(StopReason::MaxTokens),
            ProviderEvent::Complete
        ]
    );
}

#[test]
fn given_a_failed_response_when_decoded_then_the_error_has_no_status_the_code_and_the_envelope_text()
 {
    let mut decoder = plain_decoder();
    let events = feed(
        &mut decoder,
        &failed("rate_limit_exceeded", "Slow \"down\""),
    );
    let info = error_of(&events);
    assert_eq!(info.status(), None);
    assert_eq!(info.code(), Some("rate_limit_exceeded"));
    assert_eq!(
        info.text(),
        "{\"error\":{\"message\":\"Slow \\\"down\\\"\",\"type\":null,\"param\":null,\"code\":\"rate_limit_exceeded\"}}"
    );
    assert_eq!(info.retry_after(), None);
}

#[test]
fn given_a_failed_response_with_usage_and_model_when_decoded_then_usage_model_error_come_in_this_order()
 {
    let mut decoder = plain_decoder();
    let mut event = failed("server_error", "boom");
    event["response"]["usage"] = usage_object(7, 8);
    let events = feed(&mut decoder, &event);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], usage(7, 8));
    assert_eq!(events[1], model("gpt-5.1-2026-01-01"));
    assert!(matches!(events[2], ProviderEvent::Error(_)));
}

#[test]
fn given_a_failed_response_without_an_error_object_when_decoded_then_the_text_says_so_and_there_is_no_code()
 {
    for error in [json!(null), json!("a text"), json!([])] {
        let mut decoder = plain_decoder();
        let mut event = failed("x", "y");
        event["response"]["error"] = error;
        let info = error_of(&feed(&mut decoder, &event));
        assert_eq!(info.text(), "response.failed holds no error");
        assert_eq!(info.code(), None);
        assert_eq!(info.status(), None);
    }
    let mut decoder = plain_decoder();
    let mut event = failed("x", "y");
    event["response"]
        .as_object_mut()
        .expect("response")
        .remove("error");
    assert_eq!(
        error_of(&feed(&mut decoder, &event)).text(),
        "response.failed holds no error"
    );
}

#[test]
fn given_a_failed_error_whose_code_is_not_a_text_when_decoded_then_the_code_is_null_in_the_envelope_and_absent()
 {
    let mut decoder = plain_decoder();
    let mut event = failed("x", "y");
    event["response"]["error"] = json!({ "code": 5, "message": "m" });
    let info = error_of(&feed(&mut decoder, &event));
    assert_eq!(info.code(), None);
    assert_eq!(
        info.text(),
        "{\"error\":{\"message\":\"m\",\"type\":null,\"param\":null,\"code\":null}}"
    );
}

#[test]
fn given_a_top_level_error_event_when_decoded_then_one_error_comes_with_code_and_the_envelope_text()
{
    let mut decoder = plain_decoder();
    let events = feed(
        &mut decoder,
        &error_event(
            Some("server_is_overloaded"),
            "The model is overloaded",
            Some("model"),
        ),
    );
    assert_eq!(events.len(), 1);
    let info = error_of(&events);
    assert_eq!(info.status(), None);
    assert_eq!(info.code(), Some("server_is_overloaded"));
    assert_eq!(
        info.text(),
        "{\"error\":{\"message\":\"The model is overloaded\",\"type\":null,\"param\":\"model\",\"code\":\"server_is_overloaded\"}}"
    );
}

#[test]
fn given_an_error_event_with_null_code_and_param_when_decoded_then_both_are_null_and_there_is_no_code()
 {
    let mut decoder = plain_decoder();
    let info = error_of(&feed(
        &mut decoder,
        &error_event(None, "Something went wrong", None),
    ));
    assert_eq!(info.code(), None);
    assert_eq!(
        info.text(),
        "{\"error\":{\"message\":\"Something went wrong\",\"type\":null,\"param\":null,\"code\":null}}"
    );
}

#[test]
fn given_an_error_event_with_non_text_fields_when_decoded_then_they_are_null() {
    let mut decoder = plain_decoder();
    let event = json!({ "type": "error", "code": 12, "message": ["m"], "param": {} });
    let info = error_of(&feed(&mut decoder, &event));
    assert_eq!(
        info.text(),
        "{\"error\":{\"message\":null,\"type\":null,\"param\":null,\"code\":null}}"
    );
}

#[test]
fn given_the_error_envelope_when_parsed_then_it_has_the_shape_of_an_http_refusal() {
    let mut decoder = plain_decoder();
    let info = error_of(&feed(&mut decoder, &error_event(Some("c"), "m", Some("p"))));
    let parsed: serde_json::Value = serde_json::from_str(info.text()).expect("json");
    assert_eq!(
        parsed,
        json!({ "error": { "message": "m", "type": null, "param": "p", "code": "c" } })
    );
    let dialect = crate::test_requests::dialect(br_llm_clients_core::ReasoningLevel::Off);
    assert_eq!(
        br_llm_clients_common_call::Dialect::error_code(&dialect, info.text()),
        Some("c".to_owned())
    );
}

#[test]
fn given_an_error_event_after_content_when_decoded_then_still_one_error_and_no_complete() {
    let mut decoder = plain_decoder();
    flat(&mut decoder, &[message_added(0)]);
    let events = feed(&mut decoder, &error_event(Some("server_error"), "x", None));
    assert_eq!(events.len(), 1);
}

#[test]
fn given_a_terminal_event_without_a_response_object_when_decoded_then_it_is_a_payload_error() {
    for kind in [
        "response.completed",
        "response.incomplete",
        "response.failed",
        "response.created",
    ] {
        for response in [json!(null), json!("x"), json!([])] {
            let mut decoder = plain_decoder();
            let error = feed_err(&mut decoder, &json!({ "type": kind, "response": response }));
            assert!(error.text().starts_with(kind), "{kind}");
        }
        let mut decoder = plain_decoder();
        feed_err(&mut decoder, &json!({ "type": kind }));
    }
}

#[test]
fn given_a_model_that_is_not_a_text_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    let mut event = completed("m", usage_object(1, 1));
    event["response"]["model"] = json!(7);
    feed_err(&mut decoder, &event);
}

#[test]
fn given_an_incomplete_details_that_is_not_an_object_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    let mut event = incomplete(None, usage_object(1, 1));
    event["response"]["incomplete_details"] = json!("max_output_tokens");
    feed_err(&mut decoder, &event);
}

#[test]
fn given_a_reason_that_is_not_a_text_when_decoded_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    let mut event = incomplete(None, usage_object(1, 1));
    event["response"]["incomplete_details"] = json!({ "reason": 3 });
    feed_err(&mut decoder, &event);
}

#[test]
fn given_every_terminal_event_when_it_fails_then_no_partial_events_were_given() {
    let mut decoder = plain_decoder();
    flat(
        &mut decoder,
        &[
            call_added(0, "c", "n"),
            call_done(0, "c", "n", "{}", Some("completed")),
        ],
    );
    let mut event = completed("m", usage_object(1, 1));
    event["response"]["usage"]["input_tokens"] = json!(-1);
    feed_err(&mut decoder, &event);
}
