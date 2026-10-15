use std::time::Duration;

use br_llm_clients_common_http::{BodyError, NoAuthorizer, SendError};
use tokio::io::AsyncWriteExt;

use crate::support::canned::serve_bytes;
use crate::support::raw::{RawServer, read_request, reset, wait_for_close, write_raw};
use crate::support::{client, closed_port_endpoint, guarded, post};

async fn send_error(server: &RawServer) -> SendError {
    guarded(client().send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .unwrap_err()
}

async fn read_to_error(server: &RawServer) -> BodyError {
    let mut response = guarded(client().send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    loop {
        match guarded(response.body_mut().next_chunk()).await {
            Ok(Some(_)) => {}
            Ok(None) => panic!("the cut was not detected"),
            Err(error) => return error,
        }
    }
}

fn head_of_exact_size(total: usize) -> Vec<u8> {
    let prefix = "HTTP/1.1 200 OK\r\nx-pad: ";
    let suffix = "\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
    let pad = total - prefix.len() - suffix.len();
    let mut head = prefix.as_bytes().to_vec();
    head.extend(std::iter::repeat_n(b'p', pad));
    head.extend_from_slice(suffix.as_bytes());
    assert_eq!(head.len(), total);
    head
}

#[tokio::test]
async fn given_a_closed_port_then_a_plain_connection_fails_with_connect() {
    let error = guarded(client().send(post(&closed_port_endpoint(false), "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Connect { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_closed_port_then_an_https_connection_fails_with_connect_and_not_tls() {
    let error = guarded(client().send(post(&closed_port_endpoint(true), "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Connect { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_name_that_does_not_resolve_then_the_failure_is_connect() {
    let endpoint = br_llm_clients_common_http::Endpoint::https("https://no-such-host.invalid/x")
        .expect("endpoint");
    let patient = crate::support::client_with(|c| c.connect_timeout = Duration::from_secs(25));
    let error = guarded(patient.send(post(&endpoint, "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Connect { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_server_that_answers_the_handshake_with_plain_text_then_the_failure_is_tls() {
    let server = RawServer::start(|mut stream, _| async move {
        let _ = write_raw(&mut stream, b"HTTP/1.1 400 Bad Request\r\n\r\nplain").await;
        wait_for_close(&mut stream).await;
    })
    .await;
    let endpoint = br_llm_clients_common_http::Endpoint::https(&format!(
        "https://127.0.0.1:{}/",
        server.addr.port()
    ))
    .expect("endpoint");
    let error = guarded(client().send(post(&endpoint, "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Tls { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_server_that_closes_during_the_handshake_then_the_failure_is_tls() {
    let server = RawServer::start(|mut stream, _| async move {
        let _ = stream.shutdown().await;
    })
    .await;
    let endpoint = br_llm_clients_common_http::Endpoint::https(&format!(
        "https://127.0.0.1:{}/",
        server.addr.port()
    ))
    .expect("endpoint");
    let error = guarded(client().send(post(&endpoint, "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Tls { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_body_that_ends_before_its_declared_length_then_it_is_interrupted() {
    let server = serve_bytes(
        b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\nconnection: close\r\n\r\n0123456789".to_vec(),
    )
    .await;
    assert!(matches!(
        read_to_error(&server).await,
        BodyError::Interrupted { .. }
    ));
}

#[tokio::test]
async fn given_a_chunked_body_that_ends_before_its_last_chunk_then_it_is_interrupted() {
    let server = serve_bytes(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n5\r\nhello\r\n"
            .to_vec(),
    )
    .await;
    assert!(matches!(
        read_to_error(&server).await,
        BodyError::Interrupted { .. }
    ));
}

#[tokio::test]
async fn given_a_chunk_cut_in_the_middle_then_it_is_interrupted() {
    let server = serve_bytes(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n10\r\nhello"
            .to_vec(),
    )
    .await;
    assert!(matches!(
        read_to_error(&server).await,
        BodyError::Interrupted { .. }
    ));
}

#[tokio::test]
async fn given_a_reset_in_the_middle_of_the_body_then_it_is_interrupted() {
    let server = RawServer::start(|mut stream, _| async move {
        let _ = read_request(&mut stream).await;
        let _ = write_raw(
            &mut stream,
            b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\n01234",
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        reset(stream);
    })
    .await;
    assert!(matches!(
        read_to_error(&server).await,
        BodyError::Interrupted { .. }
    ));
}

#[tokio::test]
async fn given_a_reset_before_the_headers_then_the_send_is_interrupted() {
    let server = RawServer::start(|mut stream, _| async move {
        let _ = read_request(&mut stream).await;
        reset(stream);
    })
    .await;
    let error = send_error(&server).await;
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_close_before_the_headers_then_the_send_is_interrupted() {
    let server = RawServer::start(|mut stream, _| async move {
        let _ = read_request(&mut stream).await;
        let _ = stream.shutdown().await;
    })
    .await;
    let error = send_error(&server).await;
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_close_in_the_middle_of_the_headers_then_the_send_is_interrupted() {
    let server = serve_bytes(b"HTTP/1.1 200 OK\r\nx-half: va".to_vec()).await;
    let error = send_error(&server).await;
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_head_of_exactly_64_kib_then_it_is_accepted_and_one_byte_more_is_a_protocol_error()
{
    let ok = serve_bytes(head_of_exact_size(65_536)).await;
    let response = guarded(client().send(post(&ok.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(response.status(), 200);
    let too_big = serve_bytes(head_of_exact_size(65_537)).await;
    let error = send_error(&too_big).await;
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_huge_header_block_then_it_is_a_protocol_error_and_bounded() {
    let mut wire = b"HTTP/1.1 200 OK\r\nx-pad: ".to_vec();
    wire.extend(std::iter::repeat_n(b'p', 2 * 1024 * 1024));
    wire.extend_from_slice(b"\r\ncontent-length: 0\r\n\r\n");
    let server = serve_bytes(wire).await;
    let error = send_error(&server).await;
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_head_that_never_ends_then_it_is_a_protocol_error() {
    let server = RawServer::start(|mut stream, _| async move {
        let _ = read_request(&mut stream).await;
        let _ = write_raw(&mut stream, b"HTTP/1.1 200 OK\r\n").await;
        let filler = vec![b'a'; 4096];
        loop {
            if write_raw(&mut stream, &filler).await.is_err() {
                break;
            }
        }
    })
    .await;
    let error = send_error(&server).await;
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_thousand_small_headers_then_the_response_is_accepted() {
    let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
    for i in 0..1000 {
        wire.extend_from_slice(format!("x-{i}: v\r\n").as_bytes());
    }
    wire.extend_from_slice(b"content-length: 0\r\nconnection: close\r\n\r\n");
    let server = serve_bytes(wire).await;
    let response = guarded(client().send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(response.headers().iter().count(), 1002);
}

#[tokio::test]
async fn given_a_body_with_neither_length_nor_chunks_then_a_cut_is_not_detected() {
    let server =
        serve_bytes(b"HTTP/1.1 200 OK\r\nconnection: close\r\n\r\nhalf of the answ".to_vec()).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    let collected = guarded(response.body_mut().collect(1000))
        .await
        .expect("no error is possible here");
    assert_eq!(collected.bytes, b"half of the answ");
    assert!(!collected.cut);
}
