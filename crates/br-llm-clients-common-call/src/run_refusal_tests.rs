use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::{ClientError, ReasoningLevel};

use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{failure_of, run_call, spec_for, user_request};
use crate::test_server::{TestServer, http, raw};

fn secret(value: &str) -> Secret {
    Secret::new(value).expect("secret")
}

type Row<'a> = (&'a str, &'a [(&'a str, &'a str)], ClientError);

async fn refused(
    dialect: &Arc<ScriptedDialect>,
    status: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> (ClientError, usize) {
    let owned: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    let (status, body) = (status.to_owned(), body.to_vec());
    let server = TestServer::start(move |_| {
        let headers: Vec<(&str, &str)> = owned
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        http(&status, &headers, &body)
    })
    .await;
    let spec = spec_for(dialect, server.endpoint("/v1/test"));
    (failure_of(&spec, &user_request()).await, server.count())
}

#[tokio::test]
async fn given_each_refusal_status_over_the_wire_when_run_then_the_error_follows_the_table() {
    let body = br#"{"error":{"code":"the_code"}}"#;
    let text = String::from_utf8_lossy(body).into_owned();
    let delay = [("Retry-After", "30")];
    let code = || Some("the_code".to_owned());
    let rows: Vec<Row> = vec![
        ("401 Unauthorized", &[], ClientError::Authentication),
        ("403 Forbidden", &[], ClientError::PermissionDenied),
        (
            "429 Too Many Requests",
            &delay,
            ClientError::RateLimited {
                retry_after: Some(Duration::from_secs(30)),
            },
        ),
        (
            "503 Service Unavailable",
            &delay,
            ClientError::Unavailable {
                retry_after: Some(Duration::from_secs(30)),
            },
        ),
        (
            "529 Overloaded",
            &[],
            ClientError::Unavailable { retry_after: None },
        ),
        (
            "400 Bad Request",
            &[],
            ClientError::ProviderRejected {
                status: Some(400),
                code: code(),
                body: text.clone(),
            },
        ),
        (
            "404 Not Found",
            &delay,
            ClientError::ProviderRejected {
                status: Some(404),
                code: code(),
                body: text.clone(),
            },
        ),
        (
            "500 Internal Server Error",
            &[],
            ClientError::ProviderFailed {
                status: Some(500),
                code: code(),
                body: text.clone(),
            },
        ),
        (
            "302 Found",
            &[("Location", "http://127.0.0.1:1/elsewhere")],
            ClientError::ProviderFailed {
                status: Some(302),
                code: code(),
                body: text.clone(),
            },
        ),
    ];
    for (status, headers, expected) in rows {
        let dialect = Arc::new(ScriptedDialect::default());
        let (error, count) = refused(&dialect, status, headers, body).await;
        assert_eq!(error, expected, "{status}");
        assert_eq!(count, 1, "{status}: no retry");
        assert_eq!(dialect.seen_bodies(), vec![text.clone()], "{status}");
    }
}

#[tokio::test]
async fn given_statuses_from_200_to_299_with_an_empty_body_when_run_then_they_are_replies_that_ended_without_a_stop()
 {
    for status in ["200 OK", "204 No Content", "299 Custom"] {
        let dialect = Arc::new(ScriptedDialect::default());
        let (error, count) = refused(&dialect, status, &[], b"").await;
        assert_eq!(error, ClientError::StreamTruncated, "{status}");
        assert_eq!(count, 1);
        assert!(dialect.seen_infos().is_empty(), "{status}");
    }
}

#[tokio::test]
async fn given_a_refusal_with_secrets_when_run_then_the_dialect_and_the_error_never_see_them() {
    let body = br#"{"code":"c1","msg":"key sk-aaa-longer and sk-aaa and sk-bbb"}"#;
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(move |_| http("400 Bad Request", &[], body)).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![
        secret("sk-aaa"),
        secret("sk-bbb"),
        secret("sk-aaa-longer"),
    ]);
    let error = failure_of(&spec, &user_request()).await;
    let expected = r#"{"code":"c1","msg":"key [redacted] and [redacted] and [redacted]"}"#;
    assert_eq!(dialect.seen_bodies(), vec![expected.to_owned()]);
    assert_eq!(dialect.seen_infos()[0].text(), expected);
    assert_eq!(
        error,
        ClientError::ProviderRejected {
            status: Some(400),
            code: Some("c1".to_owned()),
            body: expected.to_owned()
        }
    );
}

#[tokio::test]
async fn given_a_secret_in_the_code_of_a_refusal_when_run_then_the_code_is_cleaned() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("400 Bad", &[], br#"{"code":"sk-ccc"}"#)).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let error = failure_of(&spec.with_secrets(vec![secret("sk-ccc")]), &user_request()).await;
    let ClientError::ProviderRejected { code, .. } = error else {
        panic!("not rejected");
    };
    assert_eq!(code, Some("[redacted]".to_owned()));
}

#[tokio::test]
async fn given_a_secret_in_a_text_made_by_the_dialect_when_run_then_the_text_is_cleaned() {
    let dialect = Arc::new(ScriptedDialect {
        refine: Box::new(|_| {
            Some(ClientError::ReasoningNotSupported {
                level: ReasoningLevel::Low,
                reason: "bad sk-ddd".to_owned(),
            })
        }),
        ..ScriptedDialect::default()
    });
    let server = TestServer::start(|_| http("400 Bad", &[], b"{}")).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![secret("sk-ddd")]);
    let error = failure_of(&spec, &user_request()).await;
    assert_eq!(
        error,
        ClientError::ReasoningNotSupported {
            level: ReasoningLevel::Low,
            reason: "bad [redacted]".to_owned()
        }
    );
}

#[tokio::test]
async fn given_a_body_over_the_cap_when_run_then_the_dialect_gets_the_first_64_kib() {
    let mut body = "a".repeat(65_536).into_bytes();
    body.extend_from_slice(b"TAIL");
    let dialect = Arc::new(ScriptedDialect::default());
    let (error, _) = refused(&dialect, "500 Bad", &[], &body).await;
    assert_eq!(dialect.seen_bodies(), vec!["a".repeat(65_536)]);
    let ClientError::ProviderFailed { body, .. } = error else {
        panic!("not failed");
    };
    assert_eq!(body, "a".repeat(65_536));
}

#[tokio::test]
async fn given_a_body_far_over_the_cap_when_run_then_the_rest_is_dropped() {
    let dialect = Arc::new(ScriptedDialect::default());
    refused(&dialect, "500 Bad", &[], &vec![b'a'; 300_000]).await;
    assert_eq!(dialect.seen_bodies()[0].len(), 65_536);
}

#[tokio::test]
async fn given_a_body_of_invalid_bytes_when_run_then_each_one_becomes_a_replacement_and_the_error_is_cut()
 {
    let dialect = Arc::new(ScriptedDialect::default());
    let (error, _) = refused(&dialect, "500 Bad", &[], &vec![0xFF; 65_536]).await;
    assert_eq!(dialect.seen_bodies(), vec!["\u{FFFD}".repeat(21_845)]);
    let ClientError::ProviderFailed { body, .. } = error else {
        panic!("not failed");
    };
    assert_eq!(body, "\u{FFFD}".repeat(21_845));
}

#[tokio::test]
async fn given_a_replacement_that_grows_the_body_when_run_then_the_dialect_and_the_error_get_the_cap()
 {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("500 Bad", &[], &vec![b'S'; 65_536])).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![secret("S")]);
    let error = failure_of(&spec, &user_request()).await;
    let ClientError::ProviderFailed { body, .. } = error else {
        panic!("not failed");
    };
    assert_eq!(body.len(), 65_536);
    assert!(body.starts_with("[redacted][redacted]"));
    assert_eq!(dialect.seen_bodies(), vec![body.clone()]);
    assert_eq!(dialect.seen_infos()[0].text(), body);
}

fn broken_body(status: &str, headers: &str) -> Vec<u8> {
    format!("HTTP/1.1 {status}\r\n{headers}content-length: 100\r\n\r\n0123456789").into_bytes()
}

#[tokio::test]
async fn given_a_body_that_breaks_when_run_then_the_info_has_status_and_delay_but_no_code_and_no_text()
 {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| raw(broken_body("429 Slow", "Retry-After: 12\r\n"))).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let error = failure_of(&spec, &user_request()).await;
    assert_eq!(
        error,
        ClientError::RateLimited {
            retry_after: Some(Duration::from_secs(12))
        }
    );
    assert!(dialect.seen_bodies().is_empty());
    let info = &dialect.seen_infos()[0];
    assert_eq!(info.status(), Some(429));
    assert_eq!(info.retry_after(), Some(Duration::from_secs(12)));
    assert_eq!((info.code(), info.text()), (None, ""));
}

#[tokio::test]
async fn given_a_body_that_breaks_on_a_plain_failure_when_run_then_the_error_has_an_empty_text() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| raw(broken_body("500 Bad", ""))).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let error = failure_of(&spec, &user_request()).await;
    assert_eq!(
        error,
        ClientError::ProviderFailed {
            status: Some(500),
            code: None,
            body: String::new()
        }
    );
}

#[tokio::test]
async fn given_the_headers_of_a_reply_when_run_then_the_report_holds_the_request_id() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("500 Bad", &[("X-Request-Id", "req-42")], b"")).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let report = run_call(&spec, &user_request()).await.into_parts().0;
    assert_eq!(report.request_id(), Some("req-42"));
    let dialect = Arc::new(ScriptedDialect {
        request_id_header: "x-other-id",
        ..ScriptedDialect::default()
    });
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let report = run_call(&spec, &user_request()).await.into_parts().0;
    assert_eq!(report.request_id(), None);
}

#[tokio::test]
async fn given_a_refusal_when_run_then_the_report_holds_no_usage_no_model_and_no_first_event() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("500 Bad", &[], b"")).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let report = run_call(&spec, &user_request()).await.into_parts().0;
    assert!(report.usage().is_none());
    assert!(report.model().is_none());
    assert!(report.time_to_first_event().is_none());
}
