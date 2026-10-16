use std::future::poll_fn;
use std::time::Duration;

use br_llm_clients_common_http::{BodyError, NoAuthorizer, SendError};
use tokio::io::AsyncWriteExt;

use crate::support::canned::{serve_bytes, serve_response};
use crate::support::raw::{RawServer, read_request, write_raw};
use crate::support::{client, guarded, post};

async fn chunked_server(pieces: Vec<(Duration, Vec<u8>)>) -> RawServer {
    RawServer::start(move |mut stream, _| {
        let pieces = pieces.clone();
        async move {
            if read_request(&mut stream).await.is_err() {
                return;
            }
            let _ = write_raw(
                &mut stream,
                b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n",
            )
            .await;
            for (delay, piece) in pieces {
                tokio::time::sleep(delay).await;
                let _ = write_raw(&mut stream, format!("{:x}\r\n", piece.len()).as_bytes()).await;
                let _ = write_raw(&mut stream, &piece).await;
                let _ = write_raw(&mut stream, b"\r\n").await;
            }
            let _ = write_raw(&mut stream, b"0\r\n\r\n").await;
            let _ = stream.shutdown().await;
        }
    })
    .await
}

#[tokio::test]
async fn given_a_chunked_body_when_read_then_chunks_are_non_empty_and_in_order() {
    let pieces: Vec<(Duration, Vec<u8>)> = ["alpha", "b", "gamma-gamma", "d"]
        .iter()
        .map(|p| (Duration::from_millis(5), p.as_bytes().to_vec()))
        .collect();
    let server = chunked_server(pieces).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let mut joined = Vec::new();
    let mut count = 0;
    while let Some(chunk) = guarded(response.body_mut().next_chunk())
        .await
        .expect("chunk")
    {
        assert!(!chunk.is_empty());
        joined.extend_from_slice(&chunk);
        count += 1;
    }
    assert_eq!(joined, b"alphabgamma-gammad");
    assert!(count >= 1);
}

#[tokio::test]
async fn given_the_end_of_the_body_when_read_again_then_the_stream_gives_nothing_more() {
    let server = serve_response("200 OK", &[], b"abc").await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let body = response.body_mut();
    assert_eq!(
        body.next_chunk().await.expect("chunk").as_deref(),
        Some(b"abc".as_slice())
    );
    assert_eq!(body.next_chunk().await.expect("end"), None);
    assert_eq!(body.next_chunk().await.expect("still end"), None);
    assert_eq!(body.next_chunk().await.expect("still end"), None);
}

#[tokio::test]
async fn given_an_empty_body_when_read_then_the_first_read_is_the_end() {
    let server = serve_response("204 No Content", &[], b"").await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(response.body_mut().next_chunk().await.expect("end"), None);
}

#[tokio::test]
async fn given_a_failure_in_the_body_when_read_then_it_is_given_once_and_the_stream_is_finished() {
    let wire =
        b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\nconnection: close\r\n\r\n0123456789".to_vec();
    let server = serve_bytes(wire).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let body = response.body_mut();
    let mut received = Vec::new();
    let error = loop {
        match guarded(body.next_chunk()).await {
            Ok(Some(chunk)) => received.extend_from_slice(&chunk),
            Ok(None) => panic!("the cut was not detected"),
            Err(error) => break error,
        }
    };
    assert!(matches!(error, BodyError::Interrupted { .. }), "{error:?}");
    assert_eq!(received, b"0123456789");
    assert_eq!(body.next_chunk().await.expect("finished"), None);
    assert_eq!(body.next_chunk().await.expect("finished"), None);
}

#[tokio::test]
async fn given_a_large_body_when_read_then_the_stream_has_no_size_limit() {
    let body = vec![b'x'; 6 * 1024 * 1024];
    let server = serve_response("200 OK", &[], &body).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let mut total = 0usize;
    while let Some(chunk) = guarded(response.body_mut().next_chunk())
        .await
        .expect("chunk")
    {
        assert!(!chunk.is_empty());
        total += chunk.len();
    }
    assert_eq!(total, body.len());
}

#[tokio::test]
async fn given_the_poll_interface_when_driven_by_hand_then_it_gives_the_same_chunks() {
    let server = serve_response("200 OK", &[], b"hello").await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let first = guarded(poll_fn(|cx| response.body_mut().poll_next_chunk(cx)))
        .await
        .expect("chunk");
    assert_eq!(first.as_deref(), Some(b"hello".as_slice()));
    let end = poll_fn(|cx| response.body_mut().poll_next_chunk(cx))
        .await
        .expect("end");
    assert_eq!(end, None);
}

#[tokio::test]
async fn given_a_response_when_split_into_parts_then_status_headers_and_body_come_out() {
    let server = serve_response("201 Created", &[("X-A", "1")], b"zz").await;
    let response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let (status, headers, mut body) = response.into_parts();
    assert_eq!(status, 201);
    assert_eq!(headers.get_text("x-a"), Some("1"));
    assert_eq!(body.collect(10).await.expect("body").bytes, b"zz");
}

#[tokio::test]
async fn given_an_invalid_chunk_size_in_the_body_when_read_then_it_is_a_protocol_error() {
    let wire = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n3\r\nabc\r\nzz\r\nrest".to_vec();
    let server = serve_bytes(wire).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let error = loop {
        match guarded(response.body_mut().next_chunk()).await {
            Ok(Some(_)) => {}
            Ok(None) => panic!("not detected"),
            Err(error) => break error,
        }
    };
    assert!(matches!(error, BodyError::Protocol { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_garbage_status_line_when_sent_then_it_is_a_protocol_error() {
    for wire in [
        b"NOT HTTP AT ALL\r\n\r\n".to_vec(),
        b"HTTP/1.1 2x0 OK\r\n\r\n".to_vec(),
        b"HTTP/1.1 200 OK\r\nbad header line\r\n\r\n".to_vec(),
    ] {
        let server = serve_bytes(wire).await;
        let error = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
            .await
            .unwrap_err();
        assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
    }
}
