use std::time::Duration;

use br_llm_clients_common_http::NoAuthorizer;

use crate::support::canned::{serve_bytes, serve_response};
use crate::support::raw::RawServer;
use crate::support::{client, guarded, post};

async fn get(server: &RawServer) -> br_llm_clients_common_http::Response {
    guarded(client().send(post(&server.endpoint("/"), "x"), &NoAuthorizer))
        .await
        .expect("response")
}

#[tokio::test]
async fn given_error_statuses_when_sent_then_each_is_a_response_with_its_body() {
    for (status, number) in [
        ("400 Bad Request", 400),
        ("404 Not Found", 404),
        ("429 Too Many Requests", 429),
        ("500 Internal Server Error", 500),
        ("503 Service Unavailable", 503),
    ] {
        let server = serve_response(status, &[], b"{\"error\":\"nope\"}").await;
        let mut response = get(&server).await;
        assert_eq!(response.status(), number);
        let collected = response.body_mut().collect(1024).await.expect("body");
        assert_eq!(collected.bytes, b"{\"error\":\"nope\"}");
    }
}

#[tokio::test]
async fn given_a_redirect_when_sent_then_it_is_returned_and_never_followed() {
    let target = serve_response("200 OK", &[], b"target").await;
    for status in [
        "301 Moved Permanently",
        "302 Found",
        "303 See Other",
        "307 Temporary Redirect",
        "308 Permanent Redirect",
    ] {
        let location = target.endpoint("/elsewhere").to_string();
        let server = serve_response(status, &[("Location", &location)], b"").await;
        let response = get(&server).await;
        assert_eq!(response.status().to_string(), status[..3]);
        assert_eq!(
            response.headers().get_text("location"),
            Some(location.as_str())
        );
    }
    assert_eq!(target.accepted(), 0);
}

#[tokio::test]
async fn given_repeated_headers_when_sent_then_names_are_lower_case_and_values_keep_their_order() {
    let server = serve_response(
        "200 OK",
        &[
            ("X-Multi", "a"),
            ("x-MULTI", "b"),
            ("X-Other", "o"),
            ("X-Multi", "c"),
        ],
        b"",
    )
    .await;
    let response = get(&server).await;
    let values: Vec<&[u8]> = response.headers().get_all("X-MULTI").collect();
    assert_eq!(values, [b"a".as_slice(), b"b", b"c"]);
    assert!(
        response
            .headers()
            .names()
            .all(|n| n == n.to_ascii_lowercase())
    );
    assert!(response.headers().names().any(|n| n == "x-multi"));
    assert_eq!(response.headers().get_text("x-other"), Some("o"));
}

#[tokio::test]
async fn given_a_header_value_that_is_not_utf8_when_sent_then_bytes_are_kept_and_text_is_none() {
    let wire =
        b"HTTP/1.1 200 OK\r\nx-bytes: \xff\xfe\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
            .to_vec();
    let server = serve_bytes(wire).await;
    let response = get(&server).await;
    assert_eq!(
        response.headers().get("x-bytes"),
        Some([0xff, 0xfe].as_slice())
    );
    assert_eq!(response.headers().get_text("x-bytes"), None);
}

#[tokio::test]
async fn given_a_response_when_debugging_then_header_values_and_body_are_not_shown() {
    let server = serve_response("200 OK", &[("X-Leak", "HEADER-VALUE")], b"BODY-VALUE").await;
    let response = get(&server).await;
    let debug = format!("{response:?}");
    assert!(debug.contains("x-leak") && debug.contains("200"));
    assert!(
        !debug.contains("HEADER-VALUE") && !debug.contains("BODY-VALUE"),
        "{debug}"
    );
}

async fn retry_after_of(values: &[&str]) -> Option<Duration> {
    let headers: Vec<(&str, &str)> = values.iter().map(|v| ("Retry-After", *v)).collect();
    let server = serve_response("429 Too Many Requests", &headers, b"").await;
    get(&server).await.retry_after()
}

#[tokio::test]
async fn given_a_retry_after_in_digits_when_read_then_it_is_that_many_seconds() {
    assert_eq!(retry_after_of(&["17"]).await, Some(Duration::from_secs(17)));
    assert_eq!(retry_after_of(&["0"]).await, Some(Duration::ZERO));
}

#[tokio::test]
async fn given_a_huge_retry_after_when_read_then_it_saturates() {
    assert_eq!(
        retry_after_of(&["999999999999999999999999999999"]).await,
        Some(Duration::MAX)
    );
}

#[tokio::test]
async fn given_a_repeated_retry_after_when_read_then_the_first_value_wins() {
    assert_eq!(
        retry_after_of(&["3", "9"]).await,
        Some(Duration::from_secs(3))
    );
    assert_eq!(retry_after_of(&["soon", "9"]).await, None);
}

#[tokio::test]
async fn given_a_retry_after_date_when_read_then_a_past_date_is_zero_and_a_future_date_is_a_wait() {
    for past in [
        "Sun, 06 Nov 1994 08:49:37 GMT",
        "Sunday, 06-Nov-94 08:49:37 GMT",
        "Sun Nov  6 08:49:37 1994",
    ] {
        assert_eq!(
            retry_after_of(&[past]).await,
            Some(Duration::ZERO),
            "{past}"
        );
    }
    let future = retry_after_of(&["Fri, 31 Dec 9999 23:59:59 GMT"])
        .await
        .expect("duration");
    assert!(future > Duration::from_secs(60 * 60 * 24 * 365));
}

#[tokio::test]
async fn given_a_garbage_or_missing_retry_after_when_read_then_there_is_nothing() {
    assert_eq!(retry_after_of(&["soon"]).await, None);
    assert_eq!(retry_after_of(&["-1"]).await, None);
    assert_eq!(retry_after_of(&[]).await, None);
}
