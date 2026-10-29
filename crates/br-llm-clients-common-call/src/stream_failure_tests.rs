use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_core::ClientError;

use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{spec_for, user_request};
use crate::test_server::closed_endpoint;
use crate::test_stream::{body, dialect_ending, drive_one, drive_spec, frame, kinds};

const OPEN: [&str; 2] = ["start a text", "text a hi"];

fn with(extra: &[&str]) -> Vec<u8> {
    let lines: Vec<&str> = OPEN.iter().chain(extra).copied().collect();
    body(&[&lines])
}

fn calm() -> Arc<ScriptedDialect> {
    dialect_ending(EndScript::Events(Vec::new()))
}

fn provider_failed(code: &str, text: &str) -> ClientError {
    ClientError::ProviderFailed {
        status: None,
        code: Some(code.to_owned()),
        body: text.to_owned(),
    }
}

#[tokio::test]
async fn given_a_payload_the_decoder_cannot_read_when_read_then_the_reply_is_malformed_and_nothing_after_it_is_sent()
 {
    let dialect = calm();
    let mut bytes = with(&[]);
    bytes.extend(frame(&["bad"]));
    bytes.extend(frame(&["text a later", "stop finished"]));
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    let Err(ClientError::MalformedResponse { detail }) = driven.result else {
        panic!("not malformed");
    };
    assert!(detail.contains("bad"), "{detail}");
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec!["started", "start 1 text", "text 1 \"hi\""]
    );
    assert_eq!(dialect.log.payload_count(), 2);
    assert_eq!(dialect.log.ends(), 0);
}

#[tokio::test]
async fn given_a_failing_event_in_a_list_when_read_then_the_events_after_it_never_reach_the_sink() {
    let dialect = calm();
    let driven = drive_one(
        &dialect,
        &user_request(),
        with(&["text zz oops", "text a more", "start b thinking"]),
    )
    .await;
    assert!(matches!(
        driven.result,
        Err(ClientError::MalformedResponse { .. })
    ));
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec!["started", "start 1 text", "text 1 \"hi\""]
    );
}

#[tokio::test]
async fn given_a_failure_and_an_error_in_the_same_list_when_read_then_the_error_wins() {
    let dialect = calm();
    let driven = drive_one(
        &dialect,
        &user_request(),
        with(&[
            "text zz oops",
            "text a more",
            "error - boom - provider said so",
        ]),
    )
    .await;
    assert_eq!(
        driven.result,
        Err(provider_failed("boom", "provider said so"))
    );
}

#[tokio::test]
async fn given_a_failure_and_two_errors_in_the_same_list_when_read_then_the_first_error_wins() {
    let dialect = calm();
    let driven = drive_one(
        &dialect,
        &user_request(),
        with(&["end zz", "error - first - one", "error - second - two"]),
    )
    .await;
    assert_eq!(driven.result, Err(provider_failed("first", "one")));
}

#[tokio::test]
async fn given_an_error_event_when_another_error_follows_in_the_list_then_the_first_error_stands() {
    let dialect = calm();
    let driven = drive_one(
        &dialect,
        &user_request(),
        with(&["error - first - one", "error - second - two"]),
    )
    .await;
    assert_eq!(driven.result, Err(provider_failed("first", "one")));
}

#[tokio::test]
async fn given_a_failure_and_a_complete_before_an_error_when_read_then_the_failure_stands() {
    let dialect = calm();
    let driven = drive_one(
        &dialect,
        &user_request(),
        with(&["end zz", "complete", "error - late - no"]),
    )
    .await;
    assert!(matches!(
        driven.result,
        Err(ClientError::MalformedResponse { .. })
    ));
}

#[tokio::test]
async fn given_an_error_in_a_later_list_when_the_first_list_failed_then_it_is_never_read() {
    let dialect = calm();
    let mut bytes = with(&["end zz"]);
    bytes.extend(frame(&["error - late - no"]));
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert!(matches!(
        driven.result,
        Err(ClientError::MalformedResponse { .. })
    ));
    assert_eq!(dialect.log.payload_count(), 1);
}

#[tokio::test]
async fn given_a_failure_and_the_error_after_events_to_skip_when_read_then_only_the_error_text_counts()
 {
    let dialect = calm();
    let driven = drive_one(
        &dialect,
        &user_request(),
        with(&["end zz", "stop finished", "usage 1 1", "error 429 - 3 slow"]),
    )
    .await;
    assert_eq!(
        driven.result,
        Err(ClientError::RateLimited {
            retry_after: Some(Duration::from_secs(3))
        })
    );
    assert!(driven.record.usage.is_none());
}

#[tokio::test]
async fn given_an_error_event_when_read_then_it_stops_the_call_and_the_events_after_it_are_ignored()
{
    let dialect = calm();
    let mut bytes = with(&["error 500 - - overloaded", "text a more"]);
    bytes.extend(frame(&["stop finished"]));
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert_eq!(
        driven.result,
        Err(ClientError::ProviderFailed {
            status: Some(500),
            code: None,
            body: "overloaded".to_owned()
        })
    );
    assert_eq!(
        kinds(&driven.sink.deltas),
        vec!["started", "start 1 text", "text 1 \"hi\""]
    );
    assert_eq!(dialect.log.payload_count(), 1);
}

#[tokio::test]
async fn given_an_error_event_when_the_dialect_refines_it_then_the_refined_case_wins_and_the_delay_is_set()
 {
    let dialect = Arc::new(ScriptedDialect {
        refine: Box::new(|info| {
            (info.code() == Some("busy")).then_some(ClientError::Unavailable { retry_after: None })
        }),
        ..ScriptedDialect::default()
    });
    let driven = drive_one(&dialect, &user_request(), with(&["error 500 busy 9 wait"])).await;
    assert_eq!(
        driven.result,
        Err(ClientError::Unavailable {
            retry_after: Some(Duration::from_secs(9))
        })
    );
    let info = &dialect.seen_infos()[0];
    assert_eq!(info.status(), Some(500));
    assert_eq!(info.text(), "wait");
}

#[tokio::test]
async fn given_a_status_outside_400_to_599_in_an_error_event_when_read_then_it_is_read_as_absent() {
    for status in ["200", "399", "600", "0"] {
        let dialect = calm();
        let line = format!("error {status} c - t");
        let driven = drive_one(&dialect, &user_request(), with(&[line.as_str()])).await;
        assert_eq!(driven.result, Err(provider_failed("c", "t")), "{status}");
        assert_eq!(dialect.seen_infos()[0].status(), None, "{status}");
    }
    let dialect = calm();
    let driven = drive_one(&dialect, &user_request(), with(&["error 401 c - t"])).await;
    assert_eq!(driven.result, Err(ClientError::Authentication));
}

#[tokio::test]
async fn given_secrets_in_an_error_event_when_read_then_the_dialect_never_sees_them() {
    let dialect = calm();
    let secret = br_llm_clients_common_http::Secret::new("sk-123").expect("secret");
    let spec = spec_for(&dialect, closed_endpoint()).with_secrets(vec![secret]);
    let line = "error 500 key-sk-123 - the key sk-123 leaked";
    let chunks = [body(&[&[line]])];
    let driven = drive_spec(&spec, &dialect.log, &user_request(), &chunks).await;
    assert!(driven.result.is_err());
    let info = &dialect.seen_infos()[0];
    assert_eq!(info.code(), Some("key-[redacted]"));
    assert_eq!(info.text(), "the key [redacted] leaked");
}
