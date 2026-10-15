use std::time::Duration;

use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use hyper::body::{Body, Frame};
use hyper::client::conn::http2;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::io::{AsyncWriteExt, duplex};

use crate::classify::{Cause, classify, server_broke_http2};
use crate::support::frames::{GOAWAY, HEADERS, SETTINGS, frame, goaway_frame, read_frame};
use crate::unsent::{Unsent, must_retire};

struct Empty;

impl Body for Empty {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(None)
    }

    fn is_end_stream(&self) -> bool {
        true
    }
}

const REFUSED_STREAM: u32 = 7;
const CANCEL: u32 = 8;
const INTERNAL_ERROR: u32 = 2;
const HTTP_1_1_REQUIRED: u32 = 13;
const RST_STREAM: u8 = 3;

async fn error_after_headers(reply: Vec<u8>) -> hyper::Error {
    let (client_io, mut server_io) = duplex(64 * 1024);
    tokio::spawn(async move {
        let mut preface = [0u8; 24];
        if tokio::io::AsyncReadExt::read_exact(&mut server_io, &mut preface)
            .await
            .is_err()
        {
            return;
        }
        let _ = server_io.write_all(&frame(SETTINGS, 0, 0, &[])).await;
        while let Some(received) = read_frame(&mut server_io).await {
            if received.kind == HEADERS {
                let _ = server_io.write_all(&reply).await;
                let _ = server_io.flush().await;
                tokio::time::sleep(Duration::from_secs(5)).await;
                return;
            }
        }
    });
    let (mut sender, connection) =
        http2::handshake::<_, _, Empty>(TokioExecutor::new(), TokioIo::new(client_io))
            .await
            .expect("handshake");
    tokio::spawn(connection);
    let request = http::Request::post("http://host.example/x")
        .body(Empty)
        .expect("request");
    let outcome = tokio::time::timeout(Duration::from_secs(5), sender.send_request(request))
        .await
        .expect("no outcome");
    outcome.expect_err("the server did not answer")
}

fn goaway(last_stream: u32, code: u32) -> Vec<u8> {
    goaway_frame(last_stream, code, b"")
}

fn reset(reason: u32) -> Vec<u8> {
    frame(RST_STREAM, 0, 1, &reason.to_be_bytes())
}

#[tokio::test]
async fn given_a_remote_goaway_that_excludes_the_stream_then_the_request_is_interrupted_and_the_connection_retired()
 {
    for code in [0u32, 2] {
        let error = error_after_headers(goaway(0, code)).await;
        assert_eq!(
            classify(&error, true, true),
            Cause::Interrupted { retire: true },
            "code {code}"
        );
        assert!(!server_broke_http2(&error), "code {code}");
    }
}

#[tokio::test]
async fn given_a_request_that_a_goaway_failed_then_the_connection_is_retired_but_a_refusal_keeps_it()
 {
    let excluded = error_after_headers(goaway(0, 0)).await;
    assert!(must_retire(&Unsent::Failed(excluded, true)));
    let refused = error_after_headers(reset(REFUSED_STREAM)).await;
    assert!(!must_retire(&Unsent::Failed(refused, true)));
    assert!(must_retire(&Unsent::Violation(String::new())));
}

#[tokio::test]
async fn given_a_goaway_asking_for_http_1_1_then_the_failure_is_a_protocol_failure() {
    let error = error_after_headers(goaway(0, HTTP_1_1_REQUIRED)).await;
    assert_eq!(classify(&error, true, true), Cause::Protocol);
}

#[tokio::test]
async fn given_a_refused_stream_then_the_request_is_unprocessed_and_the_connection_kept() {
    let error = error_after_headers(reset(REFUSED_STREAM)).await;
    assert_eq!(
        classify(&error, true, true),
        Cause::Unprocessed {
            taken: true,
            retire: false
        }
    );
}

#[tokio::test]
async fn given_a_stream_reset_for_another_reason_then_the_request_is_interrupted() {
    for code in [CANCEL, INTERNAL_ERROR] {
        let error = error_after_headers(reset(code)).await;
        assert_eq!(
            classify(&error, true, true),
            Cause::Interrupted { retire: false },
            "code {code}"
        );
    }
}

#[tokio::test]
async fn given_a_frame_that_breaks_an_http2_rule_then_the_failure_is_a_protocol_failure() {
    let error = error_after_headers(frame(GOAWAY, 0, 5, &[0; 8])).await;
    assert_eq!(classify(&error, true, true), Cause::Protocol);
    assert!(server_broke_http2(&error));
}

#[test]
fn given_each_cause_then_only_unprocessed_and_goaway_interruptions_ask_to_retire() {
    assert!(Cause::Interrupted { retire: true }.retires_connection());
    assert!(
        Cause::Unprocessed {
            taken: false,
            retire: true
        }
        .retires_connection()
    );
    assert!(!Cause::Interrupted { retire: false }.retires_connection());
    assert!(
        !Cause::Unprocessed {
            taken: true,
            retire: false
        }
        .retires_connection()
    );
    assert!(!Cause::Protocol.retires_connection());
}
