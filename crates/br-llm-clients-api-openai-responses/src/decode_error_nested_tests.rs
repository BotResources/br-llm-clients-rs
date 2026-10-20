use br_llm_clients_common_call::{Dialect, ErrorInfo, ProviderEvent};
use br_llm_clients_core::{ClientError, ReasoningLevel};
use serde_json::json;

use crate::test_decoder::{feed, plain_decoder};

fn info_of(event: &serde_json::Value) -> ErrorInfo {
    let mut decoder = plain_decoder();
    let events = feed(&mut decoder, event);
    assert_eq!(events.len(), 1, "{events:?}");
    match events.into_iter().next() {
        Some(ProviderEvent::Error(info)) => info,
        other => panic!("{other:?}"),
    }
}

fn text_or_null(value: Option<&str>) -> String {
    serde_json::to_string(&value).expect("json")
}

fn envelope(message: &str, param: Option<&str>, code: Option<&str>) -> String {
    format!(
        "{{\"error\":{{\"message\":{},\"type\":null,\"param\":{},\"code\":{}}}}}",
        text_or_null(Some(message)),
        text_or_null(param),
        text_or_null(code),
    )
}

#[test]
fn given_the_openai_form_with_members_at_the_top_level_when_decoded_then_it_is_read_as_in_0_1() {
    let info = info_of(&json!({
        "type": "error", "code": "server_is_overloaded", "message": "busy", "param": "model"
    }));
    assert_eq!(info.code(), Some("server_is_overloaded"));
    assert_eq!(
        info.text(),
        "{\"error\":{\"message\":\"busy\",\"type\":null,\"param\":\"model\",\"code\":\"server_is_overloaded\"}}"
    );
}

#[test]
fn given_the_azure_form_with_members_only_under_error_when_decoded_then_the_nested_members_are_read()
 {
    let info = info_of(&json!({
        "type": "error",
        "error": { "code": "rate_limit_exceeded", "message": "slow down", "param": "input" }
    }));
    assert_eq!(info.status(), None);
    assert_eq!(info.code(), Some("rate_limit_exceeded"));
    assert_eq!(
        info.text(),
        "{\"error\":{\"message\":\"slow down\",\"type\":null,\"param\":\"input\",\"code\":\"rate_limit_exceeded\"}}"
    );
}

#[test]
fn given_the_azure_form_with_the_rate_limit_code_when_refined_then_it_is_rate_limited() {
    let info = info_of(&json!({
        "type": "error",
        "error": { "code": "rate_limit_exceeded", "message": "slow down" }
    }));
    let dialect = crate::test_requests::dialect(ReasoningLevel::Off);
    assert_eq!(
        dialect.refine_error(&info),
        Some(ClientError::RateLimited { retry_after: None })
    );
}

#[test]
fn given_a_mixed_form_when_decoded_then_a_top_level_text_wins_member_by_member() {
    let info = info_of(&json!({
        "type": "error",
        "code": "top_code",
        "message": null,
        "error": { "code": "nested_code", "message": "nested message", "param": "nested_param" }
    }));
    assert_eq!(info.code(), Some("top_code"));
    assert_eq!(
        info.text(),
        envelope("nested message", Some("nested_param"), Some("top_code"))
    );
}

#[test]
fn given_a_null_at_the_top_level_when_decoded_then_the_nested_member_replaces_it() {
    let info = info_of(&json!({
        "type": "error",
        "code": null,
        "message": "top message",
        "param": 5,
        "error": { "code": "nested_code", "message": "nested message", "param": "nested_param" }
    }));
    assert_eq!(info.code(), Some("nested_code"));
    assert_eq!(
        info.text(),
        envelope("top message", Some("nested_param"), Some("nested_code"))
    );
}

#[test]
fn given_an_error_member_that_is_not_an_object_when_decoded_then_it_is_ignored_without_failing() {
    for nested in [
        json!("boom"),
        json!(7),
        json!(null),
        json!(["x"]),
        json!(true),
    ] {
        let info = info_of(&json!({
            "type": "error", "code": "c", "message": "m", "param": null, "error": nested
        }));
        assert_eq!(info.code(), Some("c"));
        assert_eq!(info.text(), envelope("m", None, Some("c")));
    }
}

#[test]
fn given_an_error_text_and_nothing_else_when_decoded_then_all_members_are_null() {
    let info = info_of(&json!({ "type": "error", "error": "boom" }));
    assert_eq!(info.code(), None);
    assert_eq!(
        info.text(),
        "{\"error\":{\"message\":null,\"type\":null,\"param\":null,\"code\":null}}"
    );
}

#[test]
fn given_an_error_object_with_no_text_member_when_decoded_then_all_members_are_null() {
    let info = info_of(&json!({
        "type": "error", "error": { "code": 4, "message": ["m"], "param": {}, "other": "x" }
    }));
    assert_eq!(info.code(), None);
    assert_eq!(
        info.text(),
        "{\"error\":{\"message\":null,\"type\":null,\"param\":null,\"code\":null}}"
    );
}

#[test]
fn given_a_nested_object_with_other_members_when_decoded_then_only_the_three_are_read() {
    let info = info_of(&json!({
        "type": "error",
        "error": { "code": "c", "message": "m", "param": "p", "type": "invalid_request_error" }
    }));
    assert_eq!(info.text(), envelope("m", Some("p"), Some("c")));
}

#[test]
fn given_a_failed_response_when_decoded_then_it_is_read_from_the_response_error_as_in_0_1() {
    let mut decoder = plain_decoder();
    let events = feed(
        &mut decoder,
        &json!({
            "type": "response.failed",
            "response": { "error": { "code": "server_error", "message": "m" }, "error_nested": 1 }
        }),
    );
    match events.last() {
        Some(ProviderEvent::Error(info)) => {
            assert_eq!(info.code(), Some("server_error"));
            assert_eq!(info.text(), envelope("m", None, Some("server_error")));
        }
        other => panic!("{other:?}"),
    }
}
