use std::time::Duration;

use br_llm_clients_common_call::{Dialect, ErrorInfo};
use br_llm_clients_core::ClientError;

use crate::test_support::dialect;

fn code(body: &str) -> Option<String> {
    dialect().error_code(body)
}

#[test]
fn given_the_documented_error_body_when_read_then_the_code_is_the_code_member() {
    let body = r#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota","param":null,"code":"insufficient_quota"}}"#;
    assert_eq!(code(body).as_deref(), Some("insufficient_quota"));
}

#[test]
fn given_a_code_and_a_different_type_when_read_then_the_code_wins() {
    let body = r#"{"error":{"message":"m","type":"invalid_request_error","param":"model","code":"model_not_found"}}"#;
    assert_eq!(code(body).as_deref(), Some("model_not_found"));
}

#[test]
fn given_a_null_code_when_read_then_the_type_is_the_code() {
    let body =
        r#"{"error":{"message":"m","type":"invalid_request_error","param":null,"code":null}}"#;
    assert_eq!(code(body).as_deref(), Some("invalid_request_error"));
}

#[test]
fn given_no_code_member_when_read_then_the_type_is_the_code() {
    assert_eq!(
        code(r#"{"error":{"type":"server_error"}}"#).as_deref(),
        Some("server_error")
    );
}

#[test]
fn given_a_code_that_is_not_a_string_when_read_then_the_type_is_used_and_else_nothing() {
    assert_eq!(
        code(r#"{"error":{"code":429,"type":"t"}}"#).as_deref(),
        Some("t")
    );
    assert_eq!(code(r#"{"error":{"code":429,"type":7}}"#), None);
    assert_eq!(code(r#"{"error":{"code":429}}"#), None);
}

#[test]
fn given_a_body_without_an_error_object_when_read_then_there_is_no_code() {
    for body in [
        r#"{"error":"just text"}"#,
        r#"{"error":null}"#,
        r#"{"error":[{"code":"x"}]}"#,
        r#"{"code":"insufficient_quota"}"#,
        r#"{"detail":{"code":"x"}}"#,
        "[]",
        r#"["error"]"#,
        "null",
        "42",
        r#""insufficient_quota""#,
    ] {
        assert_eq!(code(body), None, "{body}");
    }
}

#[test]
fn given_a_body_that_is_not_json_or_is_cut_when_read_then_there_is_no_code_and_no_panic() {
    let full = r#"{"error":{"message":"m","type":"t","code":"c"}}"#;
    for cut in 0..full.len() {
        let _ = code(full.get(..cut).unwrap_or_default());
    }
    for body in [
        "",
        "   ",
        "<html>502</html>",
        "Bad Gateway",
        "{\"error\":{\"code\":\"x\"",
        "\u{fffd}\u{fffd}",
    ] {
        assert_eq!(code(body), None, "{body}");
    }
}

const QUOTA_CODES: [&str; 5] = [
    "insufficient_quota",
    "credit_balance_exhausted",
    "organization_spend_limit_exceeded",
    "project_spend_limit_exceeded",
    "organization_usage_limit_exceeded",
];

#[test]
fn given_any_status_when_the_code_is_one_of_the_quota_codes_then_it_is_quota_exceeded() {
    for code in QUOTA_CODES {
        for status in [
            None,
            Some(200),
            Some(400),
            Some(402),
            Some(403),
            Some(429),
            Some(500),
            Some(503),
        ] {
            let mut info = ErrorInfo::new("body").with_code(code);
            if let Some(status) = status {
                info = info.with_status(status);
            }
            assert_eq!(
                dialect().refine_error(&info),
                Some(ClientError::QuotaExceeded),
                "{code} {status:?}"
            );
        }
    }
}

#[test]
fn given_insufficient_quota_with_a_delay_when_refined_then_it_is_still_quota_exceeded() {
    let info = ErrorInfo::new("body")
        .with_status(429)
        .with_code("insufficient_quota")
        .with_retry_after(Duration::from_secs(30));
    assert_eq!(
        dialect().refine_error(&info),
        Some(ClientError::QuotaExceeded)
    );
}

#[test]
fn given_any_other_code_when_refined_then_the_generic_table_decides() {
    for code in [
        "rate_limit_exceeded",
        "model_not_found",
        "context_length_exceeded",
        "invalid_api_key",
        "Insufficient_Quota",
        "insufficient_quota ",
        "credit_balance",
        "spend_limit_exceeded",
        "usage_limit_exceeded",
        "billing_hard_limit_reached",
        "",
    ] {
        for status in [400, 401, 429, 503] {
            let info = ErrorInfo::new("body").with_status(status).with_code(code);
            assert_eq!(dialect().refine_error(&info), None, "{code} {status}");
        }
    }
}

#[test]
fn given_no_code_when_refined_then_the_text_is_not_read() {
    let info =
        ErrorInfo::new("insufficient_quota You exceeded your current quota").with_status(429);
    assert_eq!(dialect().refine_error(&info), None);
}

#[test]
fn given_an_over_large_token_count_or_a_schema_problem_when_refined_then_no_specific_case_is_given()
{
    for (code, text) in [
        (
            "invalid_request_error",
            "max_completion_tokens is too large",
        ),
        ("invalid_request_error", "Invalid schema for function"),
        (
            "invalid_request_error",
            "Invalid schema for response_format",
        ),
    ] {
        let info = ErrorInfo::new(text).with_status(400).with_code(code);
        assert_eq!(dialect().refine_error(&info), None);
    }
}
