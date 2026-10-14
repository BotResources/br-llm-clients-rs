use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use br_llm_clients_common_http::{Endpoint, HttpClient, NoAuthorizer, Request, SendError};
use hyper::Response;

use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::raw::{RawServer, read_request, wait_for_close, write_raw};
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{client_with, guarded, post};

const LIMIT: Duration = Duration::from_millis(300);
const SLACK: Duration = Duration::from_millis(30);

fn assert_waited(started: Instant, at_least: Duration) {
    let waited = started.elapsed();
    assert!(
        waited + SLACK >= at_least,
        "returned after {waited:?}, before {at_least:?}"
    );
    assert!(waited < Duration::from_secs(8), "returned after {waited:?}");
}

async fn silent_after_request() -> RawServer {
    RawServer::start(|mut stream, _| async move {
        let _ = read_request(&mut stream).await;
        wait_for_close(&mut stream).await;
    })
    .await
}

#[tokio::test]
async fn given_a_connection_that_never_finishes_the_handshake_then_connect_timeout_applies() {
    let server =
        RawServer::start(|mut stream, _| async move { wait_for_close(&mut stream).await }).await;
    let endpoint =
        Endpoint::https(&format!("https://127.0.0.1:{}/", server.addr.port())).expect("endpoint");
    let client = client_with(|c| c.connect_timeout = LIMIT);
    let started = Instant::now();
    let error = guarded(client.send(post(&endpoint, "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(
        matches!(error, SendError::ConnectTimeout { .. }),
        "{error:?}"
    );
    assert_waited(started, LIMIT);
}

#[tokio::test]
async fn given_a_server_that_never_answers_then_headers_timeout_applies() {
    let server = silent_after_request().await;
    let client = client_with(|c| c.headers_timeout = LIMIT);
    let started = Instant::now();
    let error = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(
        matches!(error, SendError::HeadersTimeout { .. }),
        "{error:?}"
    );
    assert_waited(started, LIMIT);
}

#[tokio::test]
async fn given_a_server_that_stops_reading_the_body_then_headers_timeout_covers_the_stalled_write()
{
    let server =
        RawServer::start(|mut stream, _| async move { wait_for_close(&mut stream).await }).await;
    let client = client_with(|c| c.headers_timeout = LIMIT);
    let request = Request::new(server.endpoint("/"), vec![7u8; 48 * 1024 * 1024]);
    let started = Instant::now();
    let error = guarded(client.send(request, &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(
        matches!(error, SendError::HeadersTimeout { .. }),
        "{error:?}"
    );
    assert_waited(started, LIMIT);
}

#[tokio::test]
async fn given_a_slow_connection_setup_then_headers_timeout_starts_only_when_the_connection_is_ready()
 {
    let pki = Pki::trusted();
    let server = TlsServer::start_limited(
        pki.acceptor(&[b"http/1.1"]),
        service(|_| async { Ok(Response::new(ScriptBody::new([Step::Data(b"ok")]))) }),
        None,
        Duration::from_millis(700),
    )
    .await;
    let client = {
        let mut config = crate::support::config();
        config.headers_timeout = Duration::from_millis(300);
        config.extra_root_certificates = vec![pki.ca_pem.clone()];
        HttpClient::new(config).expect("client")
    };
    let started = Instant::now();
    let response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(response.status(), 200);
    assert!(started.elapsed() >= Duration::from_millis(650));
}

#[tokio::test]
async fn given_no_free_http2_stream_then_headers_timeout_covers_the_wait_for_one() {
    let pki = Pki::trusted();
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    let server = TlsServer::start_limited(
        pki.acceptor(&[b"h2"]),
        service(move |_| {
            let first = counter.fetch_add(1, Ordering::SeqCst) == 0;
            async move {
                let steps = if first {
                    vec![Step::Data(b"x"), Step::Stall]
                } else {
                    vec![Step::Data(b"y")]
                };
                Ok(Response::new(ScriptBody::new(steps)))
            }
        }),
        Some(1),
        Duration::ZERO,
    )
    .await;
    let client = {
        let mut config = crate::support::config();
        config.headers_timeout = LIMIT;
        config.extra_root_certificates = vec![pki.ca_pem.clone()];
        HttpClient::new(config).expect("client")
    };
    let held = guarded(client.send(post(&server.endpoint("/"), "1"), &NoAuthorizer))
        .await
        .expect("first");
    let started = Instant::now();
    let error = guarded(client.send(post(&server.endpoint("/"), "2"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(
        matches!(error, SendError::HeadersTimeout { .. }),
        "{error:?}"
    );
    assert_waited(started, LIMIT);
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    assert_eq!(server.accepted(), 1);
    drop(held);
}

pub(crate) fn stalling_body_server(
    first: &'static [u8],
    then_stall: bool,
) -> impl std::future::Future<Output = RawServer> {
    RawServer::start(move |mut stream, _| async move {
        if read_request(&mut stream).await.is_err() {
            return;
        }
        let _ = write_raw(
            &mut stream,
            b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n",
        )
        .await;
        let _ = write_raw(&mut stream, format!("{:x}\r\n", first.len()).as_bytes()).await;
        let _ = write_raw(&mut stream, first).await;
        let _ = write_raw(&mut stream, b"\r\n").await;
        if then_stall {
            wait_for_close(&mut stream).await;
        }
    })
}
