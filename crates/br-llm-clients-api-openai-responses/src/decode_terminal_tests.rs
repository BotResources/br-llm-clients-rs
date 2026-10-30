use br_llm_clients_common_call::{ErrorInfo, ProviderEvent};
use br_llm_messages::ResponseEnding;
use serde_json::{Value, json};

use crate::test_decoder::{feed, feed_err, model, plain_decoder, stop, usage};
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
            stop(ResponseEnding::finished()),
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
            stop(ResponseEnding::finished()),
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
            stop(ResponseEnding::finished()),
            ProviderEvent::Complete
        ]
    );
}

#[test]
fn given_the_incomplete_reasons_when_finished_then_the_stop_follows_the_table() {
    let table = [
        (
            Some("max_output_tokens"),
            ResponseEnding::output_exhaustion(),
        ),
        (Some("content_filter"), ResponseEnding::filtering()),
        (
            Some("steered"),
            crate::test_decoder::unclassified("steered"),
        ),
        (None, crate::test_decoder::unclassified("incomplete")),
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
    assert!(events.contains(&stop(crate::test_decoder::unclassified("incomplete"))));
}

#[test]
fn given_an_incomplete_response_with_an_empty_reason_when_finished_then_it_is_a_payload_error() {
    let mut decoder = plain_decoder();
    let error = feed_err(&mut decoder, &incomplete(Some(""), usage_object(1, 1)));
    assert!(error.text().starts_with("response.incomplete:"), "{error}");
    assert!(error.text().contains("empty"), "{error}");
}

#[test]
fn given_an_incomplete_response_with_a_null_details_or_reason_when_finished_then_the_stop_is_unclassified_incomplete()
 {
    for details in [Value::Null, json!({ "reason": null })] {
        let mut decoder = plain_decoder();
        let mut event = incomplete(None, usage_object(1, 1));
        event["response"]["incomplete_details"] = details.clone();
        let events = feed(&mut decoder, &event);
        assert!(
            events.contains(&stop(crate::test_decoder::unclassified("incomplete"))),
            "{details}"
        );
    }
}

#[test]
fn given_an_incomplete_response_whose_reason_is_not_a_text_when_finished_then_it_is_a_payload_error()
 {
    for reason in [
        json!(3),
        json!(true),
        json!(["max_output_tokens"]),
        json!({}),
    ] {
        let mut decoder = plain_decoder();
        let mut event = incomplete(None, usage_object(1, 1));
        event["response"]["incomplete_details"] = json!({ "reason": reason });
        let error = feed_err(&mut decoder, &event);
        assert!(error.text().starts_with("response.incomplete:"), "{error}");
    }
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
            stop(ResponseEnding::output_exhaustion()),
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
