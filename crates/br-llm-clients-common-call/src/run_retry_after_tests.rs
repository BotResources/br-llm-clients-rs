use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_core::ClientError;

use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{failure_of, spec_for, user_request};
use crate::test_server::{TestServer, http};

type Headers<'a> = &'a [(&'a str, &'a str)];

fn millis(count: u64) -> Option<Duration> {
    Some(Duration::from_millis(count))
}

fn cases() -> Vec<(Headers<'static>, Option<Duration>)> {
    vec![
        (
            &[("retry-after-ms", "1500"), ("retry-after", "2")],
            millis(1500),
        ),
        (
            &[("retry-after", "2"), ("Retry-After-Ms", "1500")],
            millis(1500),
        ),
        (
            &[("retry-after-ms", "abc"), ("retry-after", "2")],
            Some(Duration::from_secs(2)),
        ),
        (&[("retry-after-ms", "250")], millis(250)),
        (&[("retry-after", "2")], Some(Duration::from_secs(2))),
        (&[], None),
    ]
}

async fn refused(
    dialect: &Arc<ScriptedDialect>,
    status: &'static str,
    headers: Headers<'static>,
) -> ClientError {
    let server = TestServer::start(move |_| http(status, headers, b"{}")).await;
    let spec = spec_for(dialect, server.endpoint("/v1/test"));
    failure_of(&spec, &user_request()).await
}

#[tokio::test]
async fn given_the_delay_headers_when_a_429_is_refused_then_the_millisecond_header_wins() {
    for (headers, delay) in cases() {
        let dialect = Arc::new(ScriptedDialect::default());
        let error = refused(&dialect, "429 Slow", headers).await;
        assert_eq!(
            error,
            ClientError::RateLimited { retry_after: delay },
            "{headers:?}"
        );
        assert_eq!(dialect.seen_infos()[0].retry_after(), delay, "{headers:?}");
    }
}

#[tokio::test]
async fn given_the_delay_headers_when_a_503_is_refused_then_the_millisecond_header_wins() {
    for (headers, delay) in cases() {
        let dialect = Arc::new(ScriptedDialect::default());
        let error = refused(&dialect, "503 Busy", headers).await;
        assert_eq!(
            error,
            ClientError::Unavailable { retry_after: delay },
            "{headers:?}"
        );
    }
}

#[tokio::test]
async fn given_the_delay_headers_when_another_status_is_refused_then_the_info_holds_the_same_delay()
{
    for (headers, delay) in cases() {
        let dialect = Arc::new(ScriptedDialect::default());
        refused(&dialect, "500 Bad", headers).await;
        assert_eq!(dialect.seen_infos()[0].retry_after(), delay, "{headers:?}");
    }
}

#[tokio::test]
async fn given_a_dialect_that_gives_a_rate_limit_with_no_delay_when_the_headers_hold_one_then_the_library_sets_it()
 {
    let dialect = Arc::new(ScriptedDialect {
        refine: Box::new(|_| Some(ClientError::RateLimited { retry_after: None })),
        ..ScriptedDialect::default()
    });
    let error = refused(
        &dialect,
        "400 Bad",
        &[("retry-after-ms", "1500"), ("retry-after", "2")],
    )
    .await;
    assert_eq!(
        error,
        ClientError::RateLimited {
            retry_after: millis(1500)
        }
    );
}
