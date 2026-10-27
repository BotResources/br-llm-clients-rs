use std::sync::Arc;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::{ClientError, ReasoningLevel};

use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{failure_of, run_call, spec_for, unrenderable_request, user_request};
use crate::test_server::{TestServer, http};

async fn server() -> TestServer {
    TestServer::start(|_| http("500 Internal Server Error", &[], b"")).await
}

#[tokio::test]
async fn given_a_list_of_messages_that_breaks_a_rule_when_run_then_nothing_is_encoded_or_sent() {
    let server = server().await;
    let dialect = Arc::new(ScriptedDialect::default());
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let error = failure_of(&spec, &unrenderable_request()).await;
    assert!(
        matches!(error, ClientError::InvalidMessages { .. }),
        "{error:?}"
    );
    assert_eq!(server.count(), 0);
    assert_eq!(dialect.encode_calls(), 0);
}

#[tokio::test]
async fn given_a_refusal_of_the_dialect_when_run_then_it_is_returned_unchanged_and_nothing_is_sent()
{
    let server = server().await;
    let dialect = Arc::new(ScriptedDialect {
        encode_error: Some(ClientError::ToolsNotSupported),
        ..ScriptedDialect::default()
    });
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let error = failure_of(&spec, &user_request()).await;
    assert_eq!(error, ClientError::ToolsNotSupported);
    assert_eq!(dialect.encode_calls(), 1);
    assert_eq!(server.count(), 0);
}

#[tokio::test]
async fn given_a_secret_in_a_refusal_of_the_dialect_when_run_then_the_text_is_cleaned() {
    let server = server().await;
    let dialect = Arc::new(ScriptedDialect {
        encode_error: Some(ClientError::ReasoningNotSupported {
            level: ReasoningLevel::High,
            reason: "no TOPSECRET here".to_owned(),
        }),
        ..ScriptedDialect::default()
    });
    let spec = spec_for(&dialect, server.endpoint("/v1/test"))
        .with_secrets(vec![Secret::new("TOPSECRET").expect("secret")]);
    let error = failure_of(&spec, &user_request()).await;
    assert_eq!(
        error,
        ClientError::ReasoningNotSupported {
            level: ReasoningLevel::High,
            reason: "no [redacted] here".to_owned(),
        }
    );
}

fn pair(name: &str, value: &str) -> Vec<(String, String)> {
    vec![(name.to_owned(), value.to_owned())]
}

type Headers = Vec<(String, String)>;

fn defect_lists() -> Vec<(Headers, &'static str, &'static str)> {
    let twice = vec![
        ("x-dup-name".to_owned(), "dup-value-1".to_owned()),
        ("X-DUP-NAME".to_owned(), "dup-value-2".to_owned()),
    ];
    vec![
        (pair("bad name", "bad-value-1"), "bad name", "bad-value-1"),
        (
            pair("x-good-name", "bad\nvalue"),
            "x-good-name",
            "bad\nvalue",
        ),
        (pair("content-length", "12345"), "content-length", "12345"),
        (pair("host", "evil.example"), "host", "evil.example"),
        (twice, "x-dup-name", "dup-value-1"),
    ]
}

fn assert_no_leak(error: &ClientError, name: &str, value: &str) {
    let shown = format!("{error:?}");
    assert!(!shown.contains(name), "{shown}");
    for part in value.split('\n') {
        assert!(!shown.contains(part), "{shown}");
    }
}

#[tokio::test]
async fn given_a_header_of_the_dialect_that_is_refused_when_run_then_it_is_a_defect_and_nothing_is_sent()
 {
    for (headers, name, value) in defect_lists() {
        let server = server().await;
        let dialect = Arc::new(ScriptedDialect {
            headers,
            ..ScriptedDialect::default()
        });
        let spec = spec_for(&dialect, server.endpoint("/v1/test"));
        let error = failure_of(&spec, &user_request()).await;
        let ClientError::ProviderRejected { status, code, body } = &error else {
            panic!("not a defect: {error:?}");
        };
        assert_eq!((status, code), (&None, &None));
        assert!(body.contains("the dialect"), "{body}");
        assert!(!body.contains("the provider"), "{body}");
        assert_no_leak(&error, name, value);
        assert_eq!(server.count(), 0, "{name}");
    }
}

#[tokio::test]
async fn given_a_header_of_the_provider_that_is_refused_when_run_then_it_is_a_defect_and_nothing_is_sent()
 {
    for (headers, name, value) in defect_lists() {
        let server = server().await;
        let dialect = Arc::new(ScriptedDialect::default());
        let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_extra_headers(headers);
        let error = failure_of(&spec, &user_request()).await;
        let ClientError::ProviderRejected { status, code, body } = &error else {
            panic!("not a defect: {error:?}");
        };
        assert_eq!((status, code), (&None, &None));
        assert!(body.contains("the provider"), "{body}");
        assert!(!body.contains("the dialect"), "{body}");
        assert_no_leak(&error, name, value);
        assert_eq!(server.count(), 0, "{name}");
    }
}

#[tokio::test]
async fn given_headers_from_the_dialect_and_the_provider_when_run_then_the_wire_holds_the_merge() {
    let server = server().await;
    let dialect = Arc::new(ScriptedDialect {
        headers: vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("x-shared".to_owned(), "from-dialect".to_owned()),
            ("x-only-dialect".to_owned(), "d".to_owned()),
            ("Accept".to_owned(), "application/json".to_owned()),
        ],
        ..ScriptedDialect::default()
    });
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_extra_headers(vec![
        ("X-Shared".to_owned(), "from-provider".to_owned()),
        ("anthropic-version".to_owned(), "2023".to_owned()),
        ("ACCEPT".to_owned(), "text/plain".to_owned()),
    ]);
    run_call(&spec, &user_request()).await;
    let sent = server.requests();
    let [received] = sent.as_slice() else {
        panic!("one request expected");
    };
    assert_eq!(received.request_line, "POST /v1/test HTTP/1.1");
    assert_eq!(received.header_values("accept"), ["text/event-stream"]);
    assert_eq!(received.header_values("x-shared"), ["from-provider"]);
    assert_eq!(received.header_values("x-only-dialect"), ["d"]);
    assert_eq!(received.header_values("content-type"), ["application/json"]);
    assert_eq!(received.header_values("anthropic-version"), ["2023"]);
    assert_eq!(received.body, dialect.body);
}

#[tokio::test]
async fn given_the_same_header_name_in_both_lists_when_run_then_it_is_not_a_defect() {
    let server = server().await;
    let dialect = Arc::new(ScriptedDialect::default());
    let spec = spec_for(&dialect, server.endpoint("/v1/test"))
        .with_extra_headers(pair("Content-Type", "application/x-other"));
    let error = failure_of(&spec, &user_request()).await;
    assert!(
        matches!(
            error,
            ClientError::ProviderFailed {
                status: Some(500),
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(
        server.requests()[0].header_values("content-type"),
        ["application/x-other"]
    );
}

#[tokio::test]
async fn given_an_accept_from_the_dialect_only_when_run_then_the_wire_holds_the_event_stream_accept_once()
 {
    let server = server().await;
    let dialect = Arc::new(ScriptedDialect {
        headers: pair("Accept", "application/json"),
        ..ScriptedDialect::default()
    });
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    run_call(&spec, &user_request()).await;
    assert_eq!(
        server.requests()[0].header_values("accept"),
        ["text/event-stream"]
    );
}

#[tokio::test]
async fn given_an_accept_from_the_provider_only_when_run_then_the_wire_holds_the_event_stream_accept_once()
 {
    let server = server().await;
    let dialect = Arc::new(ScriptedDialect::default());
    let spec = spec_for(&dialect, server.endpoint("/v1/test"))
        .with_extra_headers(pair("aCCept", "text/plain"));
    run_call(&spec, &user_request()).await;
    assert_eq!(
        server.requests()[0].header_values("accept"),
        ["text/event-stream"]
    );
}

fn twice(name: &str, other: &str) -> Headers {
    vec![
        (name.to_owned(), "1".to_owned()),
        (other.to_owned(), "2".to_owned()),
    ]
}

#[tokio::test]
async fn given_a_defect_in_a_header_that_is_replaced_anyway_when_run_then_it_is_still_a_defect() {
    let cases: Vec<(Headers, Headers, &str)> = vec![
        (pair("accept", "bad\nvalue"), Vec::new(), "the dialect"),
        (twice("accept", "ACCEPT"), Vec::new(), "the dialect"),
        (twice("x-a", "X-A"), pair("x-a", "3"), "the dialect"),
        (Vec::new(), pair("ACCEPT", "bad\nvalue"), "the provider"),
        (Vec::new(), twice("accept", "Accept"), "the provider"),
    ];
    for (dialect_headers, provider_headers, origin) in cases {
        let server = server().await;
        let dialect = Arc::new(ScriptedDialect {
            headers: dialect_headers,
            ..ScriptedDialect::default()
        });
        let spec =
            spec_for(&dialect, server.endpoint("/v1/test")).with_extra_headers(provider_headers);
        let error = failure_of(&spec, &user_request()).await;
        let ClientError::ProviderRejected { body, .. } = &error else {
            panic!("not a defect: {error:?}");
        };
        assert!(body.contains(origin), "{body}");
        assert_eq!(server.count(), 0);
    }
}
