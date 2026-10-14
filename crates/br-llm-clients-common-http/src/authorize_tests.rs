use bytes::Bytes;
use http::header::{HeaderName, HeaderValue};

use crate::authorize::merge;
use crate::authorizer::AuthorizedHeader;
use crate::endpoint::Endpoint;
use crate::prepare::{Prepared, prepare};
use crate::request::Request;
use crate::secret::Secret;
use crate::send_error::SendError;

fn prepared() -> Prepared {
    let endpoint = Endpoint::https("https://h:8443/v1?q=1").expect("endpoint");
    let request = Request::new(endpoint, "BODY")
        .with_header("content-type", "application/json")
        .and_then(|r| r.with_header("x-keep", "kept"))
        .expect("headers");
    prepare(request, &HeaderValue::from_static("agent/1")).expect("prepared")
}

fn header(name: &str, value: &str) -> AuthorizedHeader {
    AuthorizedHeader::new(name, Secret::new(value).expect("secret"))
}

fn value_of<'a>(prepared: &'a Prepared, name: &str) -> Option<&'a HeaderValue> {
    let wanted = HeaderName::from_bytes(name.as_bytes()).expect("name");
    prepared
        .headers
        .iter()
        .find(|(n, _)| *n == wanted)
        .map(|(_, v)| v)
}

fn invalid(returned: Vec<AuthorizedHeader>) -> String {
    match merge(&mut prepared(), returned) {
        Err(SendError::InvalidAuthorization { text }) => text,
        other => panic!("expected InvalidAuthorization, got {other:?}"),
    }
}

#[test]
fn given_a_request_when_preparing_then_the_library_headers_come_first_and_are_exact() {
    let p = prepared();
    let names: Vec<&str> = p.headers.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "host",
            "user-agent",
            "accept-encoding",
            "content-length",
            "content-type",
            "x-keep"
        ]
    );
    assert_eq!(value_of(&p, "host").expect("host"), "h:8443");
    assert_eq!(value_of(&p, "accept-encoding").expect("ae"), "identity");
    assert_eq!(value_of(&p, "content-length").expect("cl"), "4");
    assert_eq!(value_of(&p, "user-agent").expect("ua"), "agent/1");
    assert_eq!(p.path_and_query, "/v1?q=1");
    assert_eq!(p.body, Bytes::from_static(b"BODY"));
}

#[test]
fn given_a_user_agent_on_the_request_when_preparing_then_it_replaces_the_configured_one_once() {
    let endpoint = Endpoint::https("https://h/").expect("endpoint");
    let request = Request::new(endpoint, "")
        .with_header("User-Agent", "mine/1")
        .expect("header");
    let p = prepare(request, &HeaderValue::from_static("agent/1")).expect("prepared");
    let agents: Vec<_> = p
        .headers
        .iter()
        .filter(|(n, _)| n == "user-agent")
        .collect();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].1, "mine/1");
    assert_eq!(value_of(&p, "content-length").expect("cl"), "0");
}

#[test]
fn given_a_new_header_when_merging_then_it_is_appended_and_marked_sensitive() {
    let mut p = prepared();
    merge(&mut p, vec![header("X-Api-Key", "sk-1")]).expect("merged");
    let last = p.headers.last().expect("last");
    assert_eq!(last.0.as_str(), "x-api-key");
    assert_eq!(last.1, "sk-1");
    assert!(last.1.is_sensitive());
}

#[test]
fn given_a_header_with_the_name_of_a_request_header_when_merging_then_it_replaces_it_in_place() {
    let mut p = prepared();
    let before = p.headers.len();
    merge(
        &mut p,
        vec![
            header("Content-Type", "application/x-other"),
            header("user-agent", "signed/1"),
        ],
    )
    .expect("merged");
    assert_eq!(p.headers.len(), before);
    assert_eq!(
        value_of(&p, "content-type").expect("ct"),
        "application/x-other"
    );
    assert_eq!(value_of(&p, "user-agent").expect("ua"), "signed/1");
    assert_eq!(
        p.headers
            .iter()
            .filter(|(n, _)| n == "content-type")
            .count(),
        1
    );
}

#[test]
fn given_no_header_when_merging_then_nothing_changes() {
    let mut p = prepared();
    let before = p.headers.len();
    merge(&mut p, Vec::new()).expect("merged");
    assert_eq!(p.headers.len(), before);
}

#[test]
fn given_an_invalid_name_when_merging_then_the_text_names_it_without_the_value() {
    let text = invalid(vec![header("bad name", "TOP-SECRET")]);
    assert!(text.contains("bad name"));
    assert!(!text.contains("TOP-SECRET"));
}

#[test]
fn given_an_invalid_value_when_merging_then_the_text_names_the_header_without_the_value() {
    for value in ["tok\n", "tok\r\n", "a\nb-TOP-SECRET", "\u{0}TOP-SECRET"] {
        let text = invalid(vec![header("authorization", value)]);
        assert!(text.contains("authorization"), "{text}");
        assert!(
            !text.contains("TOP-SECRET") && !text.contains("tok"),
            "{text}"
        );
    }
}

#[test]
fn given_a_reserved_name_when_merging_then_it_is_refused_in_any_case() {
    for name in [
        "Host",
        "content-length",
        "Transfer-Encoding",
        "connection",
        "keep-alive",
        "Proxy-Connection",
        "upgrade",
        "TE",
        "Expect",
        "ACCEPT-ENCODING",
    ] {
        let text = invalid(vec![header(name, "TOP-SECRET")]);
        assert!(text.contains("managed by the library"), "{name}: {text}");
        assert!(!text.contains("TOP-SECRET"));
    }
}

#[test]
fn given_a_name_twice_in_any_case_when_merging_then_it_is_refused_and_nothing_is_applied() {
    let mut p = prepared();
    let before: Vec<_> = p
        .headers
        .iter()
        .map(|(n, v)| (n.clone(), v.clone()))
        .collect();
    let error = merge(
        &mut p,
        vec![header("x-a", "one-SECRET"), header("X-A", "two-SECRET")],
    )
    .unwrap_err();
    let SendError::InvalidAuthorization { text } = error else {
        panic!("wrong error")
    };
    assert!(text.to_lowercase().contains("x-a") && text.contains("twice"));
    assert!(!text.contains("SECRET"));
    let after: Vec<_> = p
        .headers
        .iter()
        .map(|(n, v)| (n.clone(), v.clone()))
        .collect();
    assert_eq!(before, after);
}

#[test]
fn given_a_long_name_when_merging_then_the_text_is_bounded() {
    let text = invalid(vec![header(&"x ".repeat(500), "v")]);
    assert!(text.len() < 200, "{}", text.len());
}

#[test]
fn given_values_when_merging_then_the_redactor_removes_each_exact_value() {
    let mut p = prepared();
    let redactor = merge(
        &mut p,
        vec![
            header("authorization", "Bearer sk-1"),
            header("x-sig", "sig-2"),
        ],
    )
    .expect("merged");
    assert_eq!(
        redactor.clean("a Bearer sk-1 b sig-2"),
        "a [redacted] b [redacted]"
    );
}
