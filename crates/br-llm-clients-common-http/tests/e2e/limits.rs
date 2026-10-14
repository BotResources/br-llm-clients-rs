use br_llm_clients_common_http::{NoAuthorizer, SendError};

use crate::support::canned::serve_bytes;
use crate::support::h2_headers::HeadersServer;
use crate::support::pki::Pki;
use crate::support::{client, guarded, post, trusting_client};

async fn h1_lines(count: usize) -> Result<usize, SendError> {
    let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
    for i in 0..count.saturating_sub(2) {
        wire.extend_from_slice(format!("x{i}: v\r\n").as_bytes());
    }
    wire.extend_from_slice(b"content-length: 0\r\nconnection: close\r\n\r\n");
    let server = serve_bytes(wire).await;
    let response = guarded(client().send(post(&server.endpoint("/"), "a"), &NoAuthorizer)).await?;
    Ok(response.headers().len())
}

async fn h2_lines(count: usize) -> Result<usize, SendError> {
    let headers = (0..count)
        .map(|i| (format!("x{i}"), "v".to_owned()))
        .collect();
    let server = HeadersServer::start(headers).await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let response = guarded(client.send(post(&server.endpoint(), "a"), &NoAuthorizer)).await?;
    Ok(response.headers().len())
}

async fn h2_list_of(rfc_size: usize) -> Result<u16, SendError> {
    let value_len = rfc_size - 42 - ("x".len() + 32);
    let server = HeadersServer::start(vec![("x".to_owned(), "a".repeat(value_len))]).await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let response = guarded(client.send(post(&server.endpoint(), "a"), &NoAuthorizer)).await?;
    Ok(response.status())
}

#[tokio::test]
async fn given_1024_header_lines_over_http1_then_the_response_is_accepted_and_1025_is_refused() {
    assert_eq!(h1_lines(1024).await.expect("1024 lines"), 1024);
    let error = h1_lines(1025).await.unwrap_err();
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
}

#[tokio::test]
async fn given_1024_header_lines_over_http2_then_the_response_is_accepted_and_1025_is_refused() {
    assert_eq!(h2_lines(1024).await.expect("1024 lines"), 1024);
    let error = h2_lines(1025).await.unwrap_err();
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
}

#[tokio::test]
async fn given_an_http2_head_of_exactly_64_kib_then_it_is_accepted_and_one_byte_more_is_refused() {
    assert_eq!(h2_list_of(65_536).await.expect("exactly 64 KiB"), 200);
    let error = h2_list_of(65_537).await.unwrap_err();
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
}

#[tokio::test]
async fn given_an_http2_head_far_beyond_the_limit_then_it_is_refused_with_protocol() {
    let error = h2_list_of(500_000).await.unwrap_err();
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
}
