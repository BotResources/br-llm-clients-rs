use std::sync::Arc;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::ClientError;

use crate::secret_log::SecretLog;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{failure_of, spec_for, user_request};
use crate::test_server::{TestServer, http};

const CAP: usize = 65_536;
const SECRET: &str = "QZXVsecret7788";

fn secret(value: &str) -> Secret {
    Secret::new(value).expect("secret")
}

fn filler(length: usize) -> Vec<u8> {
    vec![b'a'; length]
}

fn crossing(secret: &str, start: usize, total: usize) -> Vec<u8> {
    let mut body = filler(CAP.saturating_sub(start));
    body.extend_from_slice(secret.as_bytes());
    body.resize(total, b'a');
    body
}

struct Seen {
    error_body: String,
    dialect_text: String,
}

async fn refused(listed: &[&str], logged: &[&str], body: Vec<u8>) -> Seen {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(move |_| http("500 Bad", &[], &body)).await;
    let log = SecretLog::new();
    for value in logged {
        log.record(&secret(value));
    }
    let spec = spec_for(&dialect, server.endpoint("/v1/test"))
        .with_secrets(listed.iter().map(|value| secret(value)).collect())
        .with_secret_log(log);
    let ClientError::ProviderFailed { body, .. } = failure_of(&spec, &user_request()).await else {
        panic!("not failed");
    };
    let dialect_text = dialect.seen_infos()[0].text().to_owned();
    Seen {
        error_body: body,
        dialect_text,
    }
}

#[tokio::test]
async fn given_a_secret_that_crosses_the_cut_when_run_then_no_part_of_it_stays_anywhere() {
    for start in [1, SECRET.len() / 2, SECRET.len() - 1] {
        let seen = refused(&[SECRET], &[], crossing(SECRET, start, CAP + 500)).await;
        let expected = "a".repeat(CAP - start);
        assert_eq!(seen.error_body, expected, "cut after {start} bytes");
        assert_eq!(seen.dialect_text, expected, "cut after {start} bytes");
    }
}

#[tokio::test]
async fn given_a_secret_wholly_before_the_cut_when_run_then_it_is_replaced() {
    let mut body = filler(100);
    body.extend_from_slice(SECRET.as_bytes());
    body.resize(CAP + 500, b'a');
    let seen = refused(&[SECRET], &[], body).await;
    let expected = format!(
        "{}[redacted]{}",
        "a".repeat(100),
        "a".repeat(CAP - 100 - 14)
    );
    assert_eq!(seen.dialect_text, expected);
    assert_eq!(seen.error_body, expected);
}

#[tokio::test]
async fn given_a_cut_body_whose_end_starts_no_secret_when_run_then_the_end_is_kept() {
    let mut body = filler(CAP + 500);
    body[CAP - 2..CAP].copy_from_slice(b"ZX");
    let seen = refused(&[SECRET], &[], body).await;
    assert_eq!(seen.dialect_text.len(), CAP);
    assert!(seen.dialect_text.ends_with("aZX"));
}

#[tokio::test]
async fn given_a_body_that_is_not_cut_when_it_ends_like_a_secret_start_then_the_end_is_kept() {
    let mut short = filler(100);
    short.extend_from_slice(b"QZXV");
    let seen = refused(&[SECRET], &[], short).await;
    assert_eq!(seen.dialect_text, format!("{}QZXV", "a".repeat(100)));
    let mut exact = filler(CAP - 4);
    exact.extend_from_slice(b"QZXV");
    let seen = refused(&[SECRET], &[], exact.clone()).await;
    assert_eq!(seen.dialect_text.as_bytes(), exact.as_slice());
}

#[tokio::test]
async fn given_two_secrets_when_the_longer_suffix_belongs_to_the_shorter_then_no_part_of_either_stays()
 {
    let longer = "QRzzzzzzzzzzz";
    let shorter = "ABQR1234";
    let mut body = filler(CAP - 4);
    body.extend_from_slice(b"ABQR");
    body.extend_from_slice(b"1234");
    body.resize(CAP + 500, b'a');
    let seen = refused(&[longer, shorter], &[], body).await;
    assert_eq!(seen.dialect_text, "a".repeat(CAP - 4));
}

#[tokio::test]
async fn given_two_secrets_with_overlapping_starts_when_run_then_the_longer_start_goes() {
    let longer = "CDEFYYYYYYYY";
    let shorter = "ABCDXXXX";
    let mut body = filler(CAP - 4);
    body.extend_from_slice(b"ABCDXXXX");
    body.resize(CAP + 500, b'a');
    let seen = refused(&[longer, shorter], &[], body).await;
    assert_eq!(seen.dialect_text, "a".repeat(CAP - 4));
}

#[tokio::test]
async fn given_a_secret_only_in_the_log_when_it_crosses_the_cut_then_no_part_of_it_stays() {
    let seen = refused(&[], &[SECRET], crossing(SECRET, 5, CAP + 500)).await;
    assert_eq!(seen.error_body, "a".repeat(CAP - 5));
    assert_eq!(seen.dialect_text, "a".repeat(CAP - 5));
}

#[tokio::test]
async fn given_a_secret_in_neither_list_nor_log_when_it_crosses_the_cut_then_its_start_stays() {
    let seen = refused(&[], &[], crossing(SECRET, 5, CAP + 500)).await;
    assert!(seen.dialect_text.ends_with("QZXVs"));
}
