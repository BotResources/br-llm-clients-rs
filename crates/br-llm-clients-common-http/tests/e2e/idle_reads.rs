use std::task::{Context, Poll, Waker};
use std::time::Duration;

use br_llm_clients_common_http::{BodyError, BodyStream, NoAuthorizer};
use bytes::Bytes;
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
    let server = slow_chunks(Duration::from_millis(2600), Duration::from_secs(60)).await;
    let client = client_with(|c| c.idle_timeout = Duration::from_millis(2000));
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    let first =
        tokio::time::timeout(Duration::from_millis(100), response.body_mut().next_chunk()).await;
    assert!(first.is_err());
    tokio::time::sleep(Duration::from_millis(2000)).await;
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

fn poll_once(body: &mut BodyStream) -> Poll<Result<Option<Bytes>, BodyError>> {
    body.poll_next_chunk(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn given_a_read_made_with_poll_next_chunk_that_stays_pending_when_it_is_polled_again_after_the_idle_timeout_then_it_fails_with_idle_timeout()
 {
    let server = slow_chunks(Duration::from_secs(60), Duration::from_secs(120)).await;
    let client = client_with(|c| c.idle_timeout = Duration::from_millis(300));
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    let started = tokio::time::Instant::now();
    assert!(poll_once(response.body_mut()).is_pending());
    tokio::time::sleep(Duration::from_millis(500)).await;
    let Poll::Ready(Err(error)) = poll_once(response.body_mut()) else {
        panic!("the second poll must end the wait that the first one started");
    };
    assert!(matches!(error, BodyError::IdleTimeout { .. }), "{error:?}");
    assert!(
        started.elapsed() >= Duration::from_millis(500),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn given_a_chunk_that_arrives_between_two_polls_then_the_wait_starts_again_from_that_chunk() {
    let server = slow_chunks(Duration::from_millis(400), Duration::from_millis(900)).await;
    let client = client_with(|c| c.idle_timeout = Duration::from_millis(600));
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert!(poll_once(response.body_mut()).is_pending());
    tokio::time::sleep(Duration::from_millis(450)).await;
    let Poll::Ready(Ok(Some(first))) = poll_once(response.body_mut()) else {
        panic!("the first chunk arrived");
    };
    assert_eq!(first.as_ref(), b"hello");
    let mut polled = poll_once(response.body_mut());
    tokio::time::sleep(Duration::from_millis(350)).await;
    for _ in 0..40 {
        if !polled.is_pending() {
            break;
        }
        polled = poll_once(response.body_mut());
        if polled.is_pending() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
    let Poll::Ready(Ok(Some(second))) = polled else {
        panic!("800 ms passed since the first poll but only 350 ms since the chunk: {polled:?}");
    };
    assert_eq!(second.as_ref(), b"world");
}
