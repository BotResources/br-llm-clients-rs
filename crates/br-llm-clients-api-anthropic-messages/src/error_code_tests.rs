use br_llm_clients_common_call::Dialect;

use crate::test_fixtures::dialect;

const NOT_FOUND: &str = r#"{
  "type": "error",
  "error": {
    "type": "not_found_error",
    "message": "The requested resource could not be found."
  },
  "request_id": "req_011CSHoEeqs5C35K2UUqR7Fy"
}"#;

const SPEND_CAP: &str = r#"{
  "type": "error",
  "error": {
    "type": "rate_limit_error",
    "message": "You have reached your API usage limits: your organization has crossed its monthly API usage threshold, set based on your organization's API tier. You will regain access on 2026-09-01 at 00:00 UTC.",
    "details": { "error_code": "enforced_spend_limit_reached" }
  },
  "request_id": "req_018EeWyXxfu5pfWkrYcMdjWG"
}"#;

#[test]
fn given_the_documented_error_bodies_when_read_then_the_code_is_the_error_type() {
    assert_eq!(
        dialect().error_code(NOT_FOUND),
        Some("not_found_error".to_owned())
    );
    assert_eq!(
        dialect().error_code(SPEND_CAP),
        Some("rate_limit_error".to_owned())
    );
}

#[test]
fn given_a_body_that_is_not_an_error_object_with_a_text_type_when_read_then_there_is_no_code() {
    for body in [
        "",
        "not json",
        "[]",
        "null",
        "\"error\"",
        "{}",
        r#"{"type":"error"}"#,
        r#"{"error":"boom"}"#,
        r#"{"error":[]}"#,
        r#"{"error":{}}"#,
        r#"{"error":{"type":5}}"#,
        r#"{"error":{"type":null}}"#,
        r#"{"type":"not_found_error"}"#,
    ] {
        assert_eq!(dialect().error_code(body), None, "{body:?}");
    }
}

#[test]
fn given_an_error_with_an_empty_type_when_read_then_the_empty_text_is_given_as_it_is() {
    assert_eq!(
        dialect().error_code(r#"{"error":{"type":""}}"#),
        Some(String::new())
    );
}

#[test]
fn given_the_same_body_when_read_twice_then_the_answer_is_identical() {
    assert_eq!(
        dialect().error_code(NOT_FOUND),
        dialect().error_code(NOT_FOUND)
    );
}
