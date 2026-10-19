use br_llm_clients_common_call::{ErrorInfo, ProviderEvent};
use br_llm_messages::StopReason;
use serde_json::json;

use crate::test_decoder::{feed, model, plain_decoder, stop, usage};
use crate::test_events::{completed, failed, incomplete, usage_object};

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
