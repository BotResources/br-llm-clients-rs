use std::time::Duration;

use br_llm_clients_common_http::{NoAuthorizer, Response};

use crate::support::canned::serve_response;
use crate::support::{client, guarded, post};

async fn answer(headers: &[(&str, &str)]) -> Response {
    let server = serve_response("429 Too Many Requests", headers, b"").await;
    guarded(client().send(post(&server.endpoint("/"), "x"), &NoAuthorizer))
        .await
        .expect("response")
}

async fn millis_of(values: &[&str]) -> Option<Duration> {
    let headers: Vec<(&str, &str)> = values.iter().map(|v| ("Retry-After-Ms", *v)).collect();
    answer(&headers).await.retry_after_ms()
}

#[tokio::test]
async fn given_a_retry_after_ms_in_digits_when_read_then_it_is_that_many_milliseconds() {
    assert_eq!(
        millis_of(&["1500"]).await,
        Some(Duration::from_millis(1500))
    );
    assert_eq!(millis_of(&["0"]).await, Some(Duration::ZERO));
    assert_eq!(
        millis_of(&[" 250 "]).await,
        Some(Duration::from_millis(250))
    );
}

#[tokio::test]
async fn given_a_huge_retry_after_ms_when_read_then_it_saturates() {
    assert_eq!(
        millis_of(&["999999999999999999999999999999"]).await,
        Some(Duration::MAX)
    );
}

#[tokio::test]
async fn given_a_repeated_retry_after_ms_when_read_then_the_first_value_wins() {
    assert_eq!(
        millis_of(&["300", "900"]).await,
        Some(Duration::from_millis(300))
    );
    assert_eq!(millis_of(&["soon", "900"]).await, None);
}

#[tokio::test]
async fn given_a_date_a_fraction_or_a_sign_when_read_then_there_is_nothing() {
    for value in [
        "Sun, 06 Nov 1994 08:49:37 GMT",
        "1.5",
        "-1",
        "+1",
        "soon",
        "",
    ] {
        assert_eq!(millis_of(&[value]).await, None, "{value:?}");
    }
    assert_eq!(millis_of(&[]).await, None);
}

#[tokio::test]
async fn given_only_retry_after_when_read_in_milliseconds_then_there_is_nothing() {
    let response = answer(&[("Retry-After", "7")]).await;
    assert_eq!(response.retry_after_ms(), None);
    assert_eq!(response.retry_after(), Some(Duration::from_secs(7)));
}

#[tokio::test]
async fn given_only_retry_after_ms_when_read_in_seconds_then_there_is_nothing() {
    let response = answer(&[("Retry-After-Ms", "7000")]).await;
    assert_eq!(response.retry_after(), None);
    assert_eq!(response.retry_after_ms(), Some(Duration::from_millis(7000)));
}

#[tokio::test]
async fn given_both_headers_when_read_then_each_answers_for_its_own_header() {
    let response = answer(&[("Retry-After", "7"), ("Retry-After-Ms", "250")]).await;
    assert_eq!(response.retry_after(), Some(Duration::from_secs(7)));
    assert_eq!(response.retry_after_ms(), Some(Duration::from_millis(250)));
}
