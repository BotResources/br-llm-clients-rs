use br_llm_clients_common_http::{BodyError, NoAuthorizer};

use crate::support::canned::{serve_bytes, serve_response};
use crate::support::{client, guarded, post};

async fn collect(body: &[u8], limit: usize) -> (Vec<u8>, bool) {
    let server = serve_response("200 OK", &[], body).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let collected = guarded(response.body_mut().collect(limit))
        .await
        .expect("collected");
    (collected.bytes, collected.cut)
}

#[tokio::test]
async fn given_a_body_shorter_than_the_limit_when_collecting_then_all_of_it_comes_and_it_is_not_cut()
 {
    assert_eq!(collect(b"abc", 100).await, (b"abc".to_vec(), false));
}

#[tokio::test]
async fn given_a_body_exactly_the_limit_when_collecting_then_it_is_not_cut() {
    assert_eq!(collect(b"abcde", 5).await, (b"abcde".to_vec(), false));
}

#[tokio::test]
async fn given_a_body_one_byte_over_the_limit_when_collecting_then_it_is_cut_at_the_limit() {
    assert_eq!(collect(b"abcdef", 5).await, (b"abcde".to_vec(), true));
}

#[tokio::test]
async fn given_a_limit_of_zero_when_collecting_then_only_a_non_empty_body_is_cut() {
    assert_eq!(collect(b"", 0).await, (Vec::new(), false));
    assert_eq!(collect(b"a", 0).await, (Vec::new(), true));
}

#[tokio::test]
async fn given_a_chunked_body_ending_exactly_at_the_limit_when_collecting_then_it_is_not_cut() {
    let wire = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n3\r\nabc\r\n3\r\ndef\r\n0\r\n\r\n".to_vec();
    let server = serve_bytes(wire).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let collected = guarded(response.body_mut().collect(6))
        .await
        .expect("collected");
    assert_eq!(
        (collected.bytes, collected.cut),
        (b"abcdef".to_vec(), false)
    );
}

#[tokio::test]
async fn given_a_chunked_body_with_more_beyond_the_limit_when_collecting_then_it_is_cut() {
    let wire = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n3\r\nabc\r\n3\r\ndef\r\n1\r\ng\r\n0\r\n\r\n".to_vec();
    let server = serve_bytes(wire).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let collected = guarded(response.body_mut().collect(6))
        .await
        .expect("collected");
    assert_eq!((collected.bytes, collected.cut), (b"abcdef".to_vec(), true));
}

#[tokio::test]
async fn given_a_body_that_fails_when_collecting_then_the_stream_error_is_returned_as_it_is() {
    let wire = b"HTTP/1.1 200 OK\r\ncontent-length: 50\r\nconnection: close\r\n\r\nshort".to_vec();
    let server = serve_bytes(wire).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let error = guarded(response.body_mut().collect(1000))
        .await
        .unwrap_err();
    assert!(matches!(error, BodyError::Interrupted { .. }), "{error:?}");
}

#[tokio::test]
async fn given_some_chunks_already_read_when_collecting_then_only_the_rest_is_collected() {
    let wire = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n3\r\nabc\r\n3\r\ndef\r\n0\r\n\r\n".to_vec();
    let server = serve_bytes(wire).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let first = guarded(response.body_mut().next_chunk())
        .await
        .expect("chunk")
        .expect("some");
    let rest = guarded(response.body_mut().collect(100))
        .await
        .expect("collected");
    let mut all = first.to_vec();
    all.extend_from_slice(&rest.bytes);
    assert_eq!(all, b"abcdef");
    assert!(!rest.cut);
}
