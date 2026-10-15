use br_llm_clients_common_http::{BodyError, NoAuthorizer, Response, SendError};

use crate::support::canned::serve_bytes;
use crate::support::frames::{DATA, frame, reset_frame, response_head};
use crate::support::h2_body::{BodyScript, BodyServer, End};
use crate::support::pki::Pki;
use crate::support::raw_h2::{Answer, Policy, RawH2};
use crate::support::{guarded, post, trusting_client};

const SHORT_TEN: &[&[u8]] = &[b"0123456789"];
const TEN_THEN_FIVE: &[&[u8]] = &[b"0123456789", b"abcde"];

async fn h2_response(script: BodyScript) -> Result<(Response, BodyServer), SendError> {
    let server = BodyServer::start(script).await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let response = guarded(client.send(post(&server.endpoint(), "x"), &NoAuthorizer)).await?;
    Ok((response, server))
}

async fn read_all(response: &mut Response) -> (Vec<u8>, Result<(), BodyError>) {
    let mut bytes = Vec::new();
    loop {
        match guarded(response.body_mut().next_chunk()).await {
            Ok(Some(chunk)) => bytes.extend_from_slice(&chunk),
            Ok(None) => return (bytes, Ok(())),
            Err(error) => return (bytes, Err(error)),
        }
    }
}

fn script(declared: Option<usize>, frames: &'static [&'static [u8]], end: End) -> BodyScript {
    BodyScript {
        declared,
        frames,
        end,
    }
}

#[tokio::test]
async fn given_an_http2_body_reset_with_no_error_before_its_declared_length_then_it_is_interrupted()
{
    let (mut response, _server) = h2_response(script(Some(100), SHORT_TEN, End::ResetNoError))
        .await
        .expect("response");
    let (bytes, outcome) = read_all(&mut response).await;
    assert_eq!(bytes, b"0123456789");
    assert!(
        matches!(outcome, Err(BodyError::Interrupted { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn given_an_http2_body_reset_with_no_error_before_any_byte_of_its_declared_length_then_it_is_interrupted()
 {
    let (mut response, _server) = h2_response(script(Some(100), &[], End::ResetNoError))
        .await
        .expect("response");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let (bytes, outcome) = read_all(&mut response).await;
    assert!(bytes.is_empty());
    assert!(
        matches!(outcome, Err(BodyError::Interrupted { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn given_an_http2_body_collected_after_a_no_error_reset_before_its_declared_length_then_collect_fails_interrupted()
 {
    let (mut response, _server) = h2_response(script(Some(100), SHORT_TEN, End::ResetNoError))
        .await
        .expect("response");
    let outcome = guarded(response.body_mut().collect(1000)).await;
    assert!(
        matches!(outcome, Err(BodyError::Interrupted { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn given_an_http2_body_that_ends_before_its_declared_length_then_it_is_interrupted() {
    let (mut response, _server) = h2_response(script(Some(100), SHORT_TEN, End::Fin))
        .await
        .expect("response");
    let (_, outcome) = read_all(&mut response).await;
    assert!(
        matches!(outcome, Err(BodyError::Interrupted { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn given_an_http2_body_of_exactly_its_declared_length_then_it_ends_cleanly() {
    for end in [End::Fin, End::ResetNoError] {
        let (mut response, _server) = h2_response(script(Some(10), SHORT_TEN, end))
            .await
            .expect("response");
        let (bytes, outcome) = read_all(&mut response).await;
        assert_eq!(bytes, b"0123456789");
        assert_eq!(outcome, Ok(()));
    }
}

#[tokio::test]
async fn given_an_http2_body_longer_than_its_declared_length_then_it_is_a_protocol_failure() {
    let (mut response, _server) = h2_response(script(Some(10), TEN_THEN_FIVE, End::Fin))
        .await
        .expect("response");
    let (_, outcome) = read_all(&mut response).await;
    assert!(
        matches!(outcome, Err(BodyError::Protocol { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn given_an_http2_body_reset_with_no_error_and_no_declared_length_then_the_cut_is_not_seen() {
    let (mut response, _server) = h2_response(script(None, SHORT_TEN, End::ResetNoError))
        .await
        .expect("response");
    let (bytes, outcome) = read_all(&mut response).await;
    assert_eq!(bytes, b"0123456789");
    assert_eq!(outcome, Ok(()));
}

async fn http1_outcome(wire: &[u8]) -> (Vec<u8>, Result<(), BodyError>) {
    let server = serve_bytes(wire.to_vec()).await;
    let client = crate::support::client();
    let mut response = guarded(client.send(post(&server.endpoint("/"), "x"), &NoAuthorizer))
        .await
        .expect("response");
    read_all(&mut response).await
}

#[tokio::test]
async fn given_an_http1_body_that_ends_before_its_declared_length_then_it_is_interrupted() {
    let (_, outcome) = http1_outcome(
        b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\nconnection: close\r\n\r\n0123456789",
    )
    .await;
    assert!(
        matches!(outcome, Err(BodyError::Interrupted { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn given_an_http1_body_of_exactly_its_declared_length_then_it_ends_cleanly() {
    let (bytes, outcome) = http1_outcome(
        b"HTTP/1.1 200 OK\r\ncontent-length: 10\r\nconnection: close\r\n\r\n0123456789",
    )
    .await;
    assert_eq!(bytes, b"0123456789");
    assert_eq!(outcome, Ok(()));
}

#[tokio::test]
async fn given_an_http1_body_longer_than_its_declared_length_then_the_extra_bytes_are_not_part_of_it()
 {
    let (bytes, outcome) = http1_outcome(
        b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\n0123456789",
    )
    .await;
    assert_eq!(bytes, b"01234");
    assert_eq!(outcome, Ok(()));
}

fn head_and_reset(stream: u32) -> Vec<u8> {
    let mut wire = response_head(stream, Some("100"), false);
    wire.extend_from_slice(&reset_frame(stream, 0));
    wire
}

fn head_and_end(stream: u32) -> Vec<u8> {
    response_head(stream, Some("100"), true)
}

fn ten_bytes_then_a_broken_rule(stream: u32) -> Vec<u8> {
    let mut wire = response_head(stream, Some("100"), false);
    wire.extend_from_slice(&frame(DATA, 0, stream, b"0123456789"));
    wire.extend_from_slice(&frame(DATA, 0, 0, b"x"));
    wire
}

async fn raw_h2_outcome(
    build: fn(u32) -> Vec<u8>,
) -> Result<(Vec<u8>, Result<(), BodyError>), SendError> {
    let server = RawH2::start(move |_| Policy {
        answer: Answer::Frames(build),
        ..Policy::healthy()
    })
    .await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let mut response = guarded(client.send(post(&server.endpoint(), "x"), &NoAuthorizer)).await?;
    Ok(read_all(&mut response).await)
}

#[tokio::test]
async fn given_a_reset_with_no_error_that_arrives_with_the_headers_of_a_declared_body_then_it_is_interrupted()
 {
    let (bytes, outcome) = raw_h2_outcome(head_and_reset).await.expect("response");
    assert!(bytes.is_empty());
    assert!(
        matches!(outcome, Err(BodyError::Interrupted { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn given_headers_that_end_the_stream_with_a_declared_length_then_the_send_fails_with_protocol()
 {
    let outcome = raw_h2_outcome(head_and_end).await;
    assert!(
        matches!(outcome, Err(SendError::Protocol { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn given_a_broken_http2_rule_in_the_middle_of_a_declared_body_then_it_stays_a_protocol_failure()
 {
    let (_, outcome) = raw_h2_outcome(ten_bytes_then_a_broken_rule)
        .await
        .expect("response");
    assert!(
        matches!(outcome, Err(BodyError::Protocol { .. })),
        "{outcome:?}"
    );
}
