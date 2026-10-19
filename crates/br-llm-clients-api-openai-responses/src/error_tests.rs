use std::time::Duration;

use br_llm_clients_common_call::{Dialect, ErrorInfo};
use br_llm_clients_core::{ClientError, ReasoningLevel};

use crate::test_requests::dialect;

fn refine(info: &ErrorInfo) -> Option<ClientError> {
    dialect(ReasoningLevel::Off).refine_error(info)
}

fn code(body: &str) -> Option<String> {
    dialect(ReasoningLevel::Off).error_code(body)
}

fn info(code: &str) -> ErrorInfo {
    ErrorInfo::new("the text").with_code(code)
}

#[test]
fn given_an_http_error_body_with_a_code_when_read_then_the_code_is_given() {
    let body = r#"{"error":{"message":"Rate limit reached","type":"rate_limit_error","param":null,"code":"rate_limit_exceeded"}}"#;
    assert_eq!(code(body), Some("rate_limit_exceeded".to_owned()));
}

#[test]
fn given_a_body_with_a_null_code_and_a_type_when_read_then_the_type_is_given() {
    let body = r#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota","param":null,"code":null}}"#;
    assert_eq!(code(body), Some("insufficient_quota".to_owned()));
}

#[test]
fn given_an_empty_code_and_a_type_when_read_then_the_type_is_given() {
    let body = r#"{"error":{"code":"","type":"server_error"}}"#;
    assert_eq!(code(body), Some("server_error".to_owned()));
}

#[test]
fn given_a_code_and_a_type_when_read_then_the_code_wins() {
    let body = r#"{"error":{"code":"invalid_api_key","type":"invalid_request_error"}}"#;
    assert_eq!(code(body), Some("invalid_api_key".to_owned()));
}

#[test]
fn given_no_usable_code_or_type_when_read_then_nothing_is_given() {
    for body in [
        r#"{"error":{"code":null,"type":null}}"#,
        r#"{"error":{"code":"","type":""}}"#,
        r#"{"error":{"code":5,"type":["x"]}}"#,
        r#"{"error":{}}"#,
    ] {
        assert_eq!(code(body), None, "{body}");
    }
}

#[test]
fn given_a_body_of_another_shape_when_read_then_nothing_is_given() {
    for body in [
        "",
        "not json",
        "<html>502 Bad Gateway</html>",
        "[]",
        "null",
        "\"error\"",
        r#"{"detail":"x"}"#,
        r#"{"error":"a text"}"#,
        r#"{"error":null}"#,
        r#"{"error":[{"code":"x"}]}"#,
        r#"{"code":"top_level_code"}"#,
    ] {
        assert_eq!(code(body), None, "{body}");
    }
}

#[test]
fn given_a_quota_code_when_refined_then_it_is_quota_exceeded_whatever_the_status() {
    for name in [
        "insufficient_quota",
        "credit_balance_exhausted",
        "organization_spend_limit_exceeded",
        "project_spend_limit_exceeded",
        "organization_usage_limit_exceeded",
    ] {
        for status in [None, Some(400), Some(429), Some(500)] {
            let base = info(name);
            let with_status = match status {
                Some(status) => base.with_status(status),
                None => base,
            };
            assert_eq!(
                refine(&with_status),
                Some(ClientError::QuotaExceeded),
                "{name}"
            );
        }
    }
}

#[test]
fn given_rate_limit_codes_when_refined_then_they_are_rate_limited_with_the_delay() {
    let delay = Duration::from_secs(7);
    for name in ["rate_limit_exceeded", "slow_down"] {
        for status in [None, Some(429), Some(400)] {
            let mut subject = info(name).with_retry_after(delay);
            if let Some(status) = status {
                subject = subject.with_status(status);
            }
            assert_eq!(
                refine(&subject),
                Some(ClientError::RateLimited {
                    retry_after: Some(delay)
                }),
                "{name}"
            );
        }
        assert_eq!(
            refine(&info(name)),
            Some(ClientError::RateLimited { retry_after: None })
        );
    }
}

#[test]
fn given_the_overloaded_code_when_refined_then_it_is_unavailable_with_the_delay() {
    let delay = Duration::from_secs(3);
    for status in [None, Some(503), Some(200 + 100)] {
        let mut subject = info("server_is_overloaded").with_retry_after(delay);
        if let Some(status) = status {
            subject = subject.with_status(status);
        }
        assert_eq!(
            refine(&subject),
            Some(ClientError::Unavailable {
                retry_after: Some(delay)
            })
        );
    }
    assert_eq!(
        refine(&info("server_is_overloaded")),
        Some(ClientError::Unavailable { retry_after: None })
    );
}

#[test]
fn given_a_policy_code_when_refined_then_the_content_is_blocked() {
    for name in [
        "bio_policy",
        "misalignment_policy_violation",
        "image_content_policy_violation",
    ] {
        for status in [None, Some(400)] {
            let mut subject = info(name);
            if let Some(status) = status {
                subject = subject.with_status(status);
            }
            assert_eq!(
                refine(&subject),
                Some(ClientError::ContentBlocked),
                "{name}"
            );
        }
    }
}

#[test]
fn given_a_request_blaming_code_without_status_when_refined_then_the_request_is_rejected_with_code_and_text()
 {
    for name in [
        "invalid_prompt",
        "data_residency_mismatch",
        "invalid_image",
        "invalid_image_format",
        "invalid_base64_image",
        "invalid_image_url",
        "image_too_large",
        "image_too_small",
        "image_parse_error",
        "invalid_image_mode",
        "image_file_too_large",
        "unsupported_image_media_type",
        "empty_image_file",
        "failed_to_download_image",
        "image_file_not_found",
    ] {
        assert_eq!(
            refine(&info(name)),
            Some(ClientError::ProviderRejected {
                status: None,
                code: Some(name.to_owned()),
                body: "the text".to_owned()
            }),
            "{name}"
        );
    }
}

#[test]
fn given_a_request_blaming_code_with_a_status_when_refined_then_the_generic_table_decides() {
    for name in ["invalid_prompt", "invalid_image", "data_residency_mismatch"] {
        assert_eq!(refine(&info(name).with_status(400)), None, "{name}");
        assert_eq!(refine(&info(name).with_status(500)), None, "{name}");
    }
}

#[test]
fn given_the_failed_response_server_codes_when_refined_then_there_is_no_row() {
    for name in ["server_error", "vector_store_timeout"] {
        assert_eq!(refine(&info(name)), None, "{name}");
        assert_eq!(refine(&info(name).with_status(500)), None, "{name}");
    }
}

#[test]
fn given_a_code_this_dialect_never_gives_a_case_for_when_refined_then_there_is_no_row() {
    for name in [
        "context_length_exceeded",
        "model_not_found",
        "invalid_api_key",
        "tool_schema_rejected",
        "max_output_tokens_too_large",
        "unsupported_value",
    ] {
        assert_eq!(refine(&info(name).with_status(400)), None, "{name}");
    }
}

#[test]
fn given_no_code_when_refined_then_there_is_no_row_whatever_the_text_says() {
    let subject =
        ErrorInfo::new("rate_limit_exceeded insufficient_quota context length").with_status(429);
    assert_eq!(refine(&subject), None);
}

#[test]
fn given_a_quota_text_with_another_code_when_refined_then_the_text_is_never_used() {
    let subject = ErrorInfo::new("insufficient_quota").with_code("something_else");
    assert_eq!(refine(&subject), None);
}
