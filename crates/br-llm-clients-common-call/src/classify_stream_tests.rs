use std::time::Duration;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::ClientError;

use crate::classify::classify_stream_error;
use crate::error_info::ErrorInfo;
use crate::test_dialect::{Refine, ScriptedDialect};
use crate::text_clean::Secrets;

fn plain() -> ScriptedDialect {
    ScriptedDialect::default()
}

fn dialect(refine: Refine) -> ScriptedDialect {
    ScriptedDialect {
        refine,
        ..ScriptedDialect::default()
    }
}

fn after(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

fn stream(status: Option<u16>) -> ErrorInfo {
    let info = ErrorInfo::new("body").with_code("c");
    match status {
        Some(status) => info.with_status(status),
        None => info,
    }
}

fn classify_stream(dialect: &ScriptedDialect, info: ErrorInfo) -> ClientError {
    classify_stream_error(dialect, &Secrets::default(), info)
}

#[test]
fn given_a_stream_error_with_no_status_when_classified_then_the_provider_failed() {
    assert_eq!(
        classify_stream(&plain(), stream(None)),
        ClientError::ProviderFailed {
            status: None,
            code: Some("c".to_owned()),
            body: "body".to_owned()
        }
    );
}

#[test]
fn given_a_stream_status_outside_the_error_range_when_classified_then_it_is_read_as_absent() {
    for status in [100, 200, 204, 301, 399, 600, 999] {
        let dialect = plain();
        let error = classify_stream(&dialect, stream(Some(status)));
        assert_eq!(
            error,
            ClientError::ProviderFailed {
                status: None,
                code: Some("c".to_owned()),
                body: "body".to_owned()
            },
            "{status}"
        );
        assert_eq!(dialect.seen_infos()[0].status(), None, "{status}");
    }
}

#[test]
fn given_a_stream_status_in_the_error_range_when_classified_then_it_is_kept() {
    for status in [400, 404, 599] {
        let dialect = plain();
        classify_stream(&dialect, stream(Some(status)));
        assert_eq!(dialect.seen_infos()[0].status(), Some(status));
    }
    assert_eq!(
        classify_stream(&plain(), stream(Some(429))),
        ClientError::RateLimited { retry_after: None }
    );
    assert_eq!(
        classify_stream(&plain(), stream(Some(599))),
        ClientError::ProviderFailed {
            status: Some(599),
            code: Some("c".to_owned()),
            body: "body".to_owned()
        }
    );
}

#[test]
fn given_secrets_in_a_stream_error_when_classified_then_the_dialect_gets_clean_texts() {
    let secrets = Secrets::new(vec![Secret::new("TOPSECRET").expect("secret")]);
    let dialect = plain();
    let info = ErrorInfo::new("a TOPSECRET b")
        .with_status(500)
        .with_code("TOPSECRET-code");
    classify_stream_error(&dialect, &secrets, info);
    let seen = &dialect.seen_infos()[0];
    assert_eq!(seen.text(), "a [redacted] b");
    assert_eq!(seen.code(), Some("[redacted]-code"));
}

#[test]
fn given_long_texts_in_a_stream_error_when_classified_then_the_dialect_gets_them_cut() {
    let dialect = plain();
    let info = ErrorInfo::new("a".repeat(70_000)).with_code("b".repeat(70_000));
    classify_stream(&dialect, info);
    let seen = &dialect.seen_infos()[0];
    assert_eq!(seen.text().len(), 65_536);
    assert_eq!(seen.code().map(str::len), Some(65_536));
}

#[test]
fn given_a_delay_in_a_stream_error_when_classified_then_it_reaches_the_dialect_and_the_case() {
    let dialect = dialect(Box::new(|_| {
        Some(ClientError::Unavailable { retry_after: None })
    }));
    let info = ErrorInfo::new("b").with_retry_after(after(9));
    assert_eq!(
        classify_stream(&dialect, info),
        ClientError::Unavailable {
            retry_after: Some(after(9))
        }
    );
    assert_eq!(dialect.seen_infos()[0].retry_after(), Some(after(9)));
}

#[test]
fn given_a_stream_error_without_code_or_delay_when_classified_then_none_is_made_up() {
    let dialect = plain();
    classify_stream(&dialect, ErrorInfo::new("b"));
    let seen = &dialect.seen_infos()[0];
    assert_eq!(seen.code(), None);
    assert_eq!(seen.retry_after(), None);
}
