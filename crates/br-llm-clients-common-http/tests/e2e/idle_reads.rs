use std::time::Duration;

use br_llm_clients_common_http::{BodyError, NoAuthorizer};
use tokio::io::AsyncWriteExt;

use crate::support::raw::{RawServer, read_request, write_raw};
use crate::support::{client_with, guarded, post};

async fn slow_chunks(first_at: Duration, second_at: Duration) -> RawServer {
    RawServer::start(move |mut stream, _| async move {
        if read_request(&mut stream).await.is_err() {
            return;
        }
        let _ = write_raw(
            &mut stream,
            b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n",
        )
        .await;
        tokio::time::sleep(first_at).await;
        let _ = write_raw(&mut stream, b"5\r\nhello\r\n").await;
        tokio::time::sleep(second_at.saturating_sub(first_at)).await;
        let _ = write_raw(&mut stream, b"5\r\nworld\r\n0\r\n\r\n").await;
        let _ = stream.shutdown().await;
    })
    .await
}

#[tokio::test]
async fn given_a_read_dropped_before_data_arrived_when_reading_again_then_the_wait_starts_afresh() {
    let server = slow_chunks(Duration::from_millis(2500), Duration::from_secs(60)).await;
    let client = client_with(|c| c.idle_timeout = Duration::from_millis(1000));
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    let first =
        tokio::time::timeout(Duration::from_millis(100), response.body_mut().next_chunk()).await;
    assert!(first.is_err());
    tokio::time::sleep(Duration::from_millis(1900)).await;
    let chunk = response
        .body_mut()
        .next_chunk()
        .await
        .expect("no idle timeout");
    assert_eq!(chunk.as_deref(), Some(b"hello".as_slice()));
}

#[tokio::test]
async fn given_a_slow_server_when_a_new_read_waits_longer_than_the_limit_then_that_read_times_out()
{
    let server = slow_chunks(Duration::from_millis(400), Duration::from_secs(60)).await;
    let client = client_with(|c| c.idle_timeout = Duration::from_millis(1000));
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    let _ =
        tokio::time::timeout(Duration::from_millis(100), response.body_mut().next_chunk()).await;
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert_eq!(
        response
            .body_mut()
            .next_chunk()
            .await
            .expect("chunk")
            .as_deref(),
        Some(b"hello".as_slice())
    );
    let started = tokio::time::Instant::now();
    let error = response.body_mut().next_chunk().await.unwrap_err();
    assert!(matches!(error, BodyError::IdleTimeout { .. }), "{error:?}");
    assert!(
        started.elapsed() >= Duration::from_millis(950),
        "{:?}",
        started.elapsed()
    );
}
