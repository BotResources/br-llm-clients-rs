use std::time::Duration;

use br_llm_clients_common_call::{Dialect, ErrorInfo};
use br_llm_clients_core::{CacheRequirement, ClientError, ReasoningLevel};

use crate::test_fixtures::{dialect, dialect_with};

fn body(code: &str, message: &str) -> String {
    serde_json::json!({"type": "error", "error": {"type": code, "message": message}}).to_string()
}

fn refusal(status: u16, code: &str, message: &str) -> ErrorInfo {
    ErrorInfo::new(body(code, message))
        .with_status(status)
        .with_code(code)
}

fn refined(info: &ErrorInfo) -> Option<ClientError> {
    dialect().refine_error(info)
}

const SPEND_CAP_BODY: &str = r#"{"type":"error","error":{"type":"rate_limit_error","message":"You have reached your API usage limits: your organization has crossed its monthly API usage threshold, set based on your organization's API tier. You will regain access on 2026-09-01 at 00:00 UTC.","details":{"error_code":"enforced_spend_limit_reached"}},"request_id":"req_018EeWyXxfu5pfWkrYcMdjWG"}"#;

#[test]
fn given_the_documented_spend_cap_429_when_refined_then_it_is_quota_exceeded() {
    let info = ErrorInfo::new(SPEND_CAP_BODY)
        .with_status(429)
        .with_code("rate_limit_error");
    assert_eq!(refined(&info), Some(ClientError::QuotaExceeded));
}

#[test]
fn given_a_429_rate_limit_with_another_or_no_detail_when_refined_then_it_is_rate_limited() {
    let other_detail = SPEND_CAP_BODY.replace("enforced_spend_limit_reached", "other_detail");
    let no_details = body("rate_limit_error", "slow down");
    for text in [other_detail, no_details, "not json".to_owned()] {
        let info = ErrorInfo::new(text)
            .with_status(429)
            .with_code("rate_limit_error");
        assert_eq!(
            refined(&info),
            Some(ClientError::RateLimited { retry_after: None })
        );
    }
}

#[test]
fn given_the_spend_cap_detail_with_another_status_when_refined_then_the_table_decides() {
    let info = ErrorInfo::new(SPEND_CAP_BODY)
        .with_status(400)
        .with_code("rate_limit_error");
    assert_eq!(
        refined(&info),
        Some(ClientError::RateLimited { retry_after: None })
    );
    let info = ErrorInfo::new(SPEND_CAP_BODY).with_code("rate_limit_error");
    assert_eq!(
        refined(&info),
        Some(ClientError::RateLimited { retry_after: None })
    );
}

#[test]
fn given_a_429_with_no_code_when_refined_then_the_generic_table_applies() {
    let info = ErrorInfo::new("rate limited").with_status(429);
    assert_eq!(refined(&info), None);
}

#[test]
fn given_the_two_usage_limit_prefixes_on_a_400_when_refined_then_it_is_quota_exceeded() {
    for message in [
        "You have reached your specified API usage limits",
        "You have reached your specified API usage limits and more words",
        "You have reached your specified workspace API usage limits",
        "You have reached your specified workspace API usage limits and more words",
    ] {
        let info = refusal(400, "invalid_request_error", message);
        assert_eq!(
            refined(&info),
            Some(ClientError::QuotaExceeded),
            "{message}"
        );
    }
}

#[test]
fn given_a_prompt_too_long_message_on_a_400_when_refined_then_the_context_is_too_long_with_no_limit()
 {
    for message in [
        "prompt is too long: 103078 tokens > 102398 maximum",
        "prompt is too long",
    ] {
        let info = refusal(400, "invalid_request_error", message);
        assert_eq!(
            refined(&info),
            Some(ClientError::ContextTooLong { limit: None })
        );
    }
}

#[test]
fn given_the_three_documented_thinking_phrases_when_refined_then_it_is_reasoning_not_supported_with_the_level_of_the_settings()
 {
    let messages = [
        "\"thinking.type.disabled\" is not supported for this model. Use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior.",
        "\"thinking.type.disabled\" is not supported for this model. Thinking defaults to adaptive mode when not specified; use \"thinking.type.enabled\" with \"budget_tokens\" for extended thinking.",
        "To turn thinking off on this model, send \"thinking\": {\"type\": \"between_tools\"} instead of {\"type\": \"disabled\"}. The model does not think before responding. The short updates it writes between tool calls come back as thinking blocks.",
        "adaptive thinking is not supported on this model",
    ];
    for level in [
        ReasoningLevel::Off,
        ReasoningLevel::Extra,
        ReasoningLevel::Max,
    ] {
        for message in messages {
            let info = refusal(400, "invalid_request_error", message);
            let dialect = dialect_with(level, CacheRequirement::NotRequired);
            assert_eq!(
                dialect.refine_error(&info),
                Some(ClientError::ReasoningNotSupported {
                    level,
                    reason: message.to_owned()
                })
            );
        }
    }
}

#[test]
fn given_texts_that_only_resemble_a_rule_when_refined_then_nothing_is_decided() {
    for message in [
        "max_tokens: 200000 > 64000, which is the maximum allowed number of output tokens for claude-3-7-sonnet",
        "Error: prompt is too long",
        " prompt is too long",
        "Prompt is too long",
        "the model claude-nope does not exist",
        "thinking.type.enabled is not supported for this model",
        "\"thinking.type.enabled\" is not supported for this model. Use \"thinking.type.adaptive\"",
        "output_config.effort 'xhigh' is not supported when thinking is disabled on this model.",
        "you have reached your specified API usage limits",
        "You have reached your API usage limits",
        "This model does not support assistant message prefill. The conversation must end with a user message.",
    ] {
        let info = refusal(400, "invalid_request_error", message);
        assert_eq!(refined(&info), None, "{message}");
    }
}

#[test]
fn given_a_text_rule_when_the_status_or_the_code_differs_then_it_does_not_apply() {
    let message = "prompt is too long: 9 tokens > 8 maximum";
    assert_eq!(
        refined(&refusal(422, "invalid_request_error", message)),
        None
    );
    assert_eq!(refined(&refusal(400, "api_error", message)), None);
    let without_status =
        ErrorInfo::new(body("invalid_request_error", message)).with_code("invalid_request_error");
    assert_eq!(refined(&without_status), None);
    let without_code = ErrorInfo::new(body("invalid_request_error", message)).with_status(400);
    assert_eq!(refined(&without_code), None);
}

#[test]
fn given_a_400_whose_text_is_not_json_or_has_no_text_message_when_refined_then_no_text_rule_applies()
 {
    for text in [
        "prompt is too long".to_owned(),
        r#"{"error":{"type":"invalid_request_error","message":5}}"#.to_owned(),
        r#"{"error":{"type":"invalid_request_error"}}"#.to_owned(),
        r#"{"error":"prompt is too long"}"#.to_owned(),
        "[]".to_owned(),
    ] {
        let info = ErrorInfo::new(text)
            .with_status(400)
            .with_code("invalid_request_error");
        assert_eq!(refined(&info), None);
    }
}

#[test]
fn given_the_error_event_of_a_stream_when_refined_then_the_code_table_applies_without_a_status() {
    let info = ErrorInfo::new(body("overloaded_error", "Overloaded")).with_code("overloaded_error");
    assert_eq!(
        refined(&info),
        Some(ClientError::Unavailable { retry_after: None })
    );
}

#[test]
fn given_the_text_rule_phrases_in_an_error_event_without_status_when_refined_then_they_do_not_apply()
 {
    let info = ErrorInfo::new(body("invalid_request_error", "prompt is too long"))
        .with_code("invalid_request_error");
    assert_eq!(refined(&info), None);
}

#[test]
fn given_a_refusal_with_a_delay_when_refined_then_the_dialect_sets_no_delay() {
    for code in ["overloaded_error", "rate_limit_error"] {
        let info = refusal(429, code, "wait").with_retry_after(Duration::from_secs(30));
        let refined = refined(&info).expect("a case");
        assert!(matches!(
            refined,
            ClientError::Unavailable { retry_after: None }
                | ClientError::RateLimited { retry_after: None }
        ));
    }
}
