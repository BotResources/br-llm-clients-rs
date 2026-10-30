use serde_json::{Value, json};

use crate::test_support::{dialect, dialect_for, encoded_body, hello, options};
use crate::{MaxTokensField, ReasoningLevel};

fn member_names(body: &Value) -> Vec<String> {
    let mut names: Vec<String> = body.as_object().expect("object").keys().cloned().collect();
    names.sort();
    names
}

#[test]
fn given_the_default_field_when_encoded_then_the_body_holds_max_completion_tokens_only() {
    let body = encoded_body(&dialect(), &hello());
    assert_eq!(body["max_completion_tokens"], json!(1024));
    assert!(body.get("max_tokens").is_none());
    assert_eq!(
        member_names(&body),
        [
            "max_completion_tokens",
            "messages",
            "model",
            "reasoning_effort",
            "stream",
            "stream_options"
        ]
    );
}

#[test]
fn given_the_field_max_tokens_when_encoded_then_the_body_holds_max_tokens_only() {
    let configured = options().with_max_tokens_field(MaxTokensField::MaxTokens);
    let body = encoded_body(&dialect_for(ReasoningLevel::Off, configured), &hello());
    assert_eq!(body["max_tokens"], json!(1024));
    assert!(body.get("max_completion_tokens").is_none());
    assert_eq!(
        member_names(&body),
        [
            "max_tokens",
            "messages",
            "model",
            "reasoning_effort",
            "stream",
            "stream_options"
        ]
    );
}

#[test]
fn given_the_field_max_tokens_when_other_extra_members_exist_then_they_are_kept_beside_it() {
    let extra = json!({"temperature": 0.3})
        .as_object()
        .cloned()
        .expect("object");
    let configured = options()
        .with_max_tokens_field(MaxTokensField::MaxTokens)
        .with_extra_body(extra);
    let body = encoded_body(&dialect_for(ReasoningLevel::Off, configured), &hello());
    assert_eq!(body["max_tokens"], json!(1024));
    assert_eq!(body["temperature"], json!(0.3));
}

#[test]
fn given_the_default_field_and_a_max_tokens_extra_member_when_encoded_then_both_members_are_written()
 {
    let extra = json!({"max_tokens": 9})
        .as_object()
        .cloned()
        .expect("object");
    let configured = options().with_extra_body(extra);
    let body = encoded_body(&dialect_for(ReasoningLevel::Off, configured), &hello());
    assert_eq!(body["max_tokens"], json!(9));
    assert_eq!(body["max_completion_tokens"], json!(1024));
}
