use std::sync::Arc;

use br_llm_clients_common_http::{AuthorizerError, Endpoint, SendError};

use crate::support::raw::{RawServer, Received, read_request, write_response};
use crate::support::spy::Spy;
use crate::support::{client, guarded, post};

async fn echo_server() -> (RawServer, Arc<std::sync::Mutex<Vec<Received>>>) {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let server = RawServer::start(move |mut stream, _| {
        let sink = Arc::clone(&sink);
        async move {
            while let Ok(received) = read_request(&mut stream).await {
                sink.lock().expect("lock").push(received);
                if write_response(&mut stream, "200 OK", &[], b"ok")
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    })
    .await;
    (server, seen)
}

#[tokio::test]
async fn given_a_request_when_authorizing_then_the_authorizer_sees_exactly_what_the_server_receives()
 {
    let (server, seen) = echo_server().await;
    let endpoint = server.endpoint("/v1/m%2Fx:go?a=b%20c");
    let spy = Spy::returning(&[]);
    let request = post(&endpoint, "THE-BODY")
        .with_header("Content-Type", "application/json")
        .expect("header");
    guarded(client().send(request, &spy))
        .await
        .expect("response");
    let observed = spy.seen();
    assert_eq!(observed.len(), 1);
    let observed = &observed[0];
    let received = seen.lock().expect("lock")[0].clone();
    assert_eq!(observed.method, "POST");
    assert_eq!(
        received.request_line,
        format!("POST {} HTTP/1.1", observed.path_and_query)
    );
    assert_eq!(observed.path_and_query, "/v1/m%2Fx:go?a=b%20c");
    assert_eq!(observed.host, format!("127.0.0.1:{}", server.addr.port()));
    assert_eq!(observed.body, b"THE-BODY");
    assert_eq!(received.body, observed.body);
    let mut sent: Vec<(String, Vec<u8>)> = received
        .headers
        .iter()
        .map(|(n, v)| (n.to_ascii_lowercase(), v.clone().into_bytes()))
        .collect();
    let mut shown = observed.headers.clone();
    sent.sort();
    shown.sort();
    assert_eq!(shown, sent);
    for name in [
        "host",
        "content-length",
        "accept-encoding",
        "user-agent",
        "content-type",
    ] {
        assert!(observed.headers.iter().any(|(n, _)| n == name), "{name}");
    }
}

#[tokio::test]
async fn given_a_default_port_when_authorizing_then_the_host_has_no_port_and_a_failure_stops_before_any_connection()
 {
    for text in ["https://api.example.com:443/x", "https://api.example.com/x"] {
        let spy = Spy::failing(AuthorizerError::unavailable("stop"));
        let endpoint = Endpoint::https(text).expect("endpoint");
        let error = guarded(client().send(post(&endpoint, ""), &spy))
            .await
            .unwrap_err();
        assert!(matches!(error, SendError::Authorizer(_)));
        assert_eq!(spy.seen()[0].host, "api.example.com");
    }
    let spy = Spy::failing(AuthorizerError::unavailable("stop"));
    let endpoint = Endpoint::plain_http("http://api.example.com:80/x").expect("endpoint");
    let _ = guarded(client().send(post(&endpoint, ""), &spy)).await;
    assert_eq!(spy.seen()[0].host, "api.example.com");
    let spy = Spy::failing(AuthorizerError::unavailable("stop"));
    let endpoint = Endpoint::https("https://api.example.com:8443/x").expect("endpoint");
    let _ = guarded(client().send(post(&endpoint, ""), &spy)).await;
    assert_eq!(spy.seen()[0].host, "api.example.com:8443");
}

#[tokio::test]
async fn given_a_request_when_sent_then_the_authorizer_is_called_exactly_once() {
    let (server, _seen) = echo_server().await;
    let spy = Spy::returning(&[("x-api-key", "k")]);
    guarded(client().send(post(&server.endpoint("/"), "a"), &spy))
        .await
        .expect("response");
    assert_eq!(spy.calls(), 1);
}

#[tokio::test]
async fn given_a_send_that_fails_after_authorizing_then_the_authorizer_was_still_called_once() {
    let spy = Spy::returning(&[("x-api-key", "k")]);
    let endpoint = crate::support::closed_port_endpoint(false);
    let error = guarded(client().send(post(&endpoint, "a"), &spy))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Connect { .. }));
    assert_eq!(spy.calls(), 1);
}

#[tokio::test]
async fn given_returned_headers_when_sent_then_they_arrive_exactly_as_given() {
    let (server, seen) = echo_server().await;
    let spy = Spy::returning(&[
        ("X-Signature", "v1=ab+/=,;\"q\"  spaced"),
        ("x-date", "20260101T000000Z"),
    ]);
    guarded(client().send(post(&server.endpoint("/"), "a"), &spy))
        .await
        .expect("response");
    let received = seen.lock().expect("lock")[0].clone();
    assert_eq!(
        received.header("x-signature"),
        Some("v1=ab+/=,;\"q\"  spaced")
    );
    assert_eq!(received.header("x-date"), Some("20260101T000000Z"));
}

#[tokio::test]
async fn given_a_returned_header_with_the_name_of_a_request_header_when_sent_then_it_replaces_it() {
    let (server, seen) = echo_server().await;
    let spy = Spy::returning(&[
        ("Content-Type", "application/x-signed"),
        ("x-keep", "signed"),
    ]);
    let request = post(&server.endpoint("/"), "a")
        .with_header("content-type", "application/json")
        .and_then(|r| r.with_header("x-keep", "kept"))
        .expect("headers");
    guarded(client().send(request, &spy))
        .await
        .expect("response");
    let received = seen.lock().expect("lock")[0].clone();
    assert_eq!(received.header_count("content-type"), 1);
    assert_eq!(
        received.header("content-type"),
        Some("application/x-signed")
    );
    assert_eq!(received.header_count("x-keep"), 1);
    assert_eq!(received.header("x-keep"), Some("signed"));
}
