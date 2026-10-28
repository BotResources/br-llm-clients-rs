use std::time::Duration;

use crate::reply_parse::{Flaw, error_code, parse_reply};

fn flaw(body: &str) -> Flaw {
    parse_reply(body.as_bytes()).expect_err("flaw")
}

fn validity(body: &str) -> Option<Duration> {
    parse_reply(body.as_bytes())
        .expect("reply")
        .access_token
        .valid_for()
}

#[test]
fn given_a_body_that_is_not_a_json_object_then_it_is_refused() {
    for body in [
        "",
        "[]",
        "\"x\"",
        "7",
        "null",
        "{",
        "not json",
        "{\"access_token\":\"a\"} x",
        "\u{feff}{\"access_token\":\"a\"}",
    ] {
        assert_eq!(flaw(body), Flaw::NotAnObject, "{body}");
    }
    assert_eq!(
        parse_reply(b"{\"access_token\":\"a\",\"x\":\"\xff\"}").expect_err("utf8"),
        Flaw::NotAnObject
    );
}

#[test]
fn given_an_access_token_that_is_missing_empty_or_not_a_text_then_it_is_refused() {
    for body in [
        "{}",
        r#"{"access_token":""}"#,
        r#"{"access_token":null}"#,
        r#"{"access_token":5}"#,
        r#"{"access_token":["a"]}"#,
    ] {
        assert_eq!(flaw(body), Flaw::AccessToken, "{body}");
    }
}

#[test]
fn given_a_token_type_then_only_bearer_in_any_case_is_accepted() {
    for kind in ["Bearer", "bearer", "BEARER", "bEaReR"] {
        let body = format!(r#"{{"access_token":"a","token_type":"{kind}"}}"#);
        assert!(parse_reply(body.as_bytes()).is_ok(), "{kind}");
    }
    for body in [
        r#"{"access_token":"a","token_type":"mac"}"#,
        r#"{"access_token":"a","token_type":""}"#,
        r#"{"access_token":"a","token_type":"Bearer "}"#,
        r#"{"access_token":"a","token_type":null}"#,
        r#"{"access_token":"a","token_type":1}"#,
    ] {
        assert_eq!(flaw(body), Flaw::TokenType, "{body}");
    }
}

#[test]
fn given_expires_in_as_an_integer_number_then_it_is_the_validity() {
    assert_eq!(
        validity(r#"{"access_token":"a","expires_in":0}"#),
        Some(Duration::ZERO)
    );
    assert_eq!(
        validity(r#"{"access_token":"a","expires_in":3599}"#),
        Some(Duration::from_secs(3599))
    );
    assert_eq!(
        validity(r#"{"access_token":"a","expires_in":18446744073709551615}"#),
        Some(Duration::from_secs(u64::MAX))
    );
    assert_eq!(validity(r#"{"access_token":"a"}"#), None);
}

#[test]
fn given_expires_in_as_digits_in_a_text_then_it_is_the_validity_and_a_huge_one_saturates() {
    assert_eq!(
        validity(r#"{"access_token":"a","expires_in":"0"}"#),
        Some(Duration::ZERO)
    );
    assert_eq!(
        validity(r#"{"access_token":"a","expires_in":"007"}"#),
        Some(Duration::from_secs(7))
    );
    assert_eq!(
        validity(r#"{"access_token":"a","expires_in":"18446744073709551615"}"#),
        Some(Duration::from_secs(u64::MAX))
    );
    assert_eq!(
        validity(r#"{"access_token":"a","expires_in":"18446744073709551616"}"#),
        Some(Duration::from_secs(u64::MAX))
    );
    assert_eq!(
        validity(r#"{"access_token":"a","expires_in":"99999999999999999999999999"}"#),
        Some(Duration::from_secs(u64::MAX))
    );
}

#[test]
fn given_expires_in_that_is_not_a_whole_number_of_seconds_then_it_is_refused() {
    for value in [
        "3599.0",
        "-1",
        "1e3",
        "1.5",
        "\"\"",
        "\" 5\"",
        "\"5 \"",
        "\"+5\"",
        "\"-5\"",
        "\"5s\"",
        "\"0x10\"",
        "\"\u{661}\"",
        "null",
        "true",
        "[5]",
        "{}",
    ] {
        let body = format!(r#"{{"access_token":"a","expires_in":{value}}}"#);
        assert_eq!(flaw(&body), Flaw::ExpiresIn, "{value}");
    }
}

#[test]
fn given_an_expires_in_number_above_the_largest_u64_then_it_is_refused_because_it_cannot_be_told_from_a_decimal()
 {
    for value in [
        "18446744073709551616",
        "1.8446744073709552e19",
        "1000000000000000000000",
    ] {
        let body = format!(r#"{{"access_token":"a","expires_in":{value}}}"#);
        assert_eq!(flaw(&body), Flaw::ExpiresIn, "{value}");
    }
}

#[test]
fn given_a_refresh_token_then_it_must_be_a_text_that_is_not_empty() {
    let reply = parse_reply(br#"{"access_token":"a","refresh_token":"r"}"#).expect("reply");
    assert_eq!(reply.refresh_token.expect("refresh").expose(), "r");
    for body in [
        r#"{"access_token":"a","refresh_token":""}"#,
        r#"{"access_token":"a","refresh_token":null}"#,
        r#"{"access_token":"a","refresh_token":1}"#,
    ] {
        assert_eq!(flaw(body), Flaw::RefreshToken, "{body}");
    }
}

#[test]
fn given_other_members_then_they_are_ignored() {
    let body = r#"{"access_token":"a","scope":5,"id_token":null,"ext_expires_in":"x","nested":{"expires_in":"bad"}}"#;
    assert!(parse_reply(body.as_bytes()).is_ok());
}

#[test]
fn given_an_error_member_then_only_a_short_lowercase_code_is_read() {
    assert_eq!(
        error_code(br#"{"error":"invalid_grant"}"#).as_deref(),
        Some("invalid_grant")
    );
    assert_eq!(error_code(br#"{"error":"a"}"#).as_deref(), Some("a"));
    assert_eq!(error_code(br#"{"error":"Invalid"}"#), None);
    assert_eq!(error_code(br#"{"error":""}"#), None);
    assert_eq!(error_code(br#"{"error_description":"x"}"#), None);
    assert_eq!(error_code(b"{\"error\":\"a\"} x"), None);
    assert_eq!(error_code(b"\xff"), None);
}

#[test]
fn given_a_body_nested_deeper_than_the_json_reader_allows_then_it_is_refused_without_a_crash() {
    let deep = format!(
        r#"{{"access_token":"a","x":{}{}}}"#,
        "[".repeat(30_000),
        "]".repeat(30_000)
    );
    assert_eq!(flaw(&deep), Flaw::NotAnObject);
    let deep_error = format!(
        r#"{{"error":"invalid_grant","x":{}{}}}"#,
        "[".repeat(30_000),
        "]".repeat(30_000)
    );
    assert_eq!(error_code(deep_error.as_bytes()), None);
}
