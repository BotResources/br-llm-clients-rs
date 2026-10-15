use std::time::{Duration, Instant};

use br_llm_clients_common_http::{BodyError, NoAuthorizer, SendError};
use hyper::Response;
use tokio::io::AsyncWriteExt;

use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::raw::{RawServer, read_request, wait_for_close, write_raw};
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{client_with, guarded, post};
use crate::timeouts::stalling_body_server;

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
async fn given_a_body_that_stalls_while_the_importer_reads_then_idle_timeout_applies_and_the_stream_ends()
 {
    let server = stalling_body_server(b"abc", true).await;
    let client = client_with(|c| c.idle_timeout = LIMIT);
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        response
            .body_mut()
            .next_chunk()
            .await
            .expect("chunk")
            .as_deref(),
        Some(b"abc".as_slice())
    );
    let started = Instant::now();
    let error = guarded(response.body_mut().next_chunk()).await.unwrap_err();
    assert!(matches!(error, BodyError::IdleTimeout { .. }), "{error:?}");
    assert_waited(started, LIMIT);
    assert_eq!(
        response.body_mut().next_chunk().await.expect("finished"),
        None
    );
}

#[tokio::test]
async fn given_an_importer_that_is_not_reading_then_idle_timeout_does_not_run() {
    let server = RawServer::start(|mut stream, _| async move {
        if read_request(&mut stream).await.is_err() {
            return;
        }
        let _ = write_raw(&mut stream, b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n3\r\none\r\n").await;
        tokio::time::sleep(Duration::from_millis(450)).await;
        let _ = write_raw(&mut stream, b"3\r\ntwo\r\n0\r\n\r\n").await;
        let _ = stream.shutdown().await;
    })
    .await;
    let client = client_with(|c| c.idle_timeout = LIMIT);
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        response
            .body_mut()
            .next_chunk()
            .await
            .expect("one")
            .as_deref(),
        Some(b"one".as_slice())
    );
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert_eq!(
        response
            .body_mut()
            .next_chunk()
            .await
            .expect("two")
            .as_deref(),
        Some(b"two".as_slice())
    );
    assert_eq!(response.body_mut().next_chunk().await.expect("end"), None);
}

#[tokio::test]
async fn given_an_importer_that_waits_before_its_first_read_then_idle_timeout_does_not_count_that_wait()
 {
    let server = RawServer::start(|mut stream, _| async move {
        if read_request(&mut stream).await.is_err() {
            return;
        }
        let _ = write_raw(
            &mut stream,
            b"HTTP/1.1 200 OK\r\ncontent-length: 3\r\nconnection: close\r\n\r\nabc",
        )
        .await;
        let _ = stream.shutdown().await;
    })
    .await;
    let client = client_with(|c| c.idle_timeout = Duration::from_millis(150));
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        b"abc"
    );
}

#[tokio::test]
async fn given_chunks_that_each_arrive_within_the_idle_timeout_then_a_long_body_is_not_cut() {
    let server = RawServer::start(|mut stream, _| async move {
        if read_request(&mut stream).await.is_err() {
            return;
        }
        let _ = write_raw(
            &mut stream,
            b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n",
        )
        .await;
        for _ in 0..6 {
            tokio::time::sleep(Duration::from_millis(120)).await;
            let _ = write_raw(&mut stream, b"1\r\nx\r\n").await;
        }
        let _ = write_raw(&mut stream, b"0\r\n\r\n").await;
        let _ = stream.shutdown().await;
    })
    .await;
    let client = client_with(|c| c.idle_timeout = Duration::from_millis(400));
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        response.body_mut().collect(100).await.expect("body").bytes,
        b"xxxxxx"
    );
}

#[tokio::test]
async fn given_a_total_timeout_and_a_server_that_never_answers_then_the_send_fails_with_total_timeout()
 {
    let server = silent_after_request().await;
    let client = client_with(|c| c.total_timeout = Some(LIMIT));
    let started = Instant::now();
    let error = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::TotalTimeout { .. }), "{error:?}");
    assert_waited(started, LIMIT);
}

#[tokio::test]
async fn given_a_total_timeout_and_a_body_that_never_ends_then_the_body_fails_with_total_timeout() {
    let server = RawServer::start(|mut stream, _| async move {
        if read_request(&mut stream).await.is_err() {
            return;
        }
        let _ = write_raw(
            &mut stream,
            b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n",
        )
        .await;
        loop {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if write_raw(&mut stream, b"1\r\nx\r\n").await.is_err() {
                break;
            }
        }
    })
    .await;
    let client = client_with(|c| {
        c.total_timeout = Some(Duration::from_millis(500));
        c.idle_timeout = Duration::from_secs(5);
    });
    let started = Instant::now();
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    let error = loop {
        match guarded(response.body_mut().next_chunk()).await {
            Ok(Some(_)) => {}
            Ok(None) => panic!("the body ended"),
            Err(error) => break error,
        }
    };
    assert!(matches!(error, BodyError::TotalTimeout { .. }), "{error:?}");
    assert_waited(started, Duration::from_millis(500));
    assert_eq!(
        response.body_mut().next_chunk().await.expect("finished"),
        None
    );
}

#[tokio::test]
async fn given_a_total_timeout_that_the_response_beats_then_the_body_is_not_cut() {
    let server = crate::support::canned::serve_response("200 OK", &[], b"fast").await;
    let client = client_with(|c| c.total_timeout = Some(Duration::from_secs(5)));
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        b"fast"
    );
}

#[tokio::test]
async fn given_http2_and_a_stalled_body_then_idle_timeout_applies() {
    let pki = Pki::trusted();
    let server = TlsServer::start(
        pki.acceptor(&[b"h2"]),
        service(|_| async {
            Ok(Response::new(ScriptBody::new([
                Step::Data(b"a"),
                Step::Stall,
            ])))
        }),
    )
    .await;
    let client = {
        let mut config = crate::support::config();
        config.idle_timeout = LIMIT;
        config.extra_root_certificates = vec![pki.ca_pem.clone()];
        config.client()
    };
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        response
            .body_mut()
            .next_chunk()
            .await
            .expect("chunk")
            .as_deref(),
        Some(b"a".as_slice())
    );
    let error = guarded(response.body_mut().next_chunk()).await.unwrap_err();
    assert!(matches!(error, BodyError::IdleTimeout { .. }), "{error:?}");
}
