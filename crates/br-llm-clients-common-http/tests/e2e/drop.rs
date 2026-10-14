use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_http::{NoAuthorizer, SendError};
use tokio::sync::Notify;

use crate::support::raw::{RawServer, read_request, wait_for_close, write_raw, write_response};
use crate::support::{client, guarded, post};

async fn stalling_server(closed: Arc<Notify>) -> RawServer {
    RawServer::start(move |mut stream, _| {
        let closed = Arc::clone(&closed);
        async move {
            if read_request(&mut stream).await.is_err() {
                return;
            }
            let _ = write_raw(
                &mut stream,
                b"HTTP/1.1 200 OK\r\ncontent-length: 1000\r\n\r\npartial",
            )
            .await;
            wait_for_close(&mut stream).await;
            closed.notify_one();
        }
    })
    .await
}

async fn wait_closed(closed: &Notify) {
    tokio::time::timeout(Duration::from_secs(10), closed.notified())
        .await
        .expect("the server never saw the connection close");
}

#[tokio::test]
async fn given_a_stream_dropped_in_the_middle_of_a_response_then_the_connection_is_closed() {
    let closed = Arc::new(Notify::new());
    let server = stalling_server(Arc::clone(&closed)).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    let chunk = guarded(response.body_mut().next_chunk())
        .await
        .expect("chunk");
    assert_eq!(chunk.as_deref(), Some(b"partial".as_slice()));
    drop(response);
    wait_closed(&closed).await;
}

#[tokio::test]
async fn given_a_response_dropped_before_any_read_then_the_connection_is_closed() {
    let closed = Arc::new(Notify::new());
    let server = stalling_server(Arc::clone(&closed)).await;
    let response = guarded(client().send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    drop(response);
    wait_closed(&closed).await;
}

#[tokio::test]
async fn given_a_drop_in_the_middle_of_a_response_then_the_next_request_uses_another_connection() {
    let closed = Arc::new(Notify::new());
    let server = RawServer::start({
        let closed = Arc::clone(&closed);
        move |mut stream, index| {
            let closed = Arc::clone(&closed);
            async move {
                if read_request(&mut stream).await.is_err() {
                    return;
                }
                if index == 0 {
                    let _ = write_raw(
                        &mut stream,
                        b"HTTP/1.1 200 OK\r\ncontent-length: 1000\r\n\r\npartial",
                    )
                    .await;
                    wait_for_close(&mut stream).await;
                    closed.notify_one();
                } else {
                    let _ = write_response(&mut stream, "200 OK", &[], b"second").await;
                    wait_for_close(&mut stream).await;
                }
            }
        }
    })
    .await;
    let client = client();
    let mut first = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    first.body_mut().next_chunk().await.expect("chunk");
    drop(first);
    let mut second = guarded(client.send(post(&server.endpoint("/"), "b"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        second.body_mut().collect(100).await.expect("body").bytes,
        b"second"
    );
    assert_eq!(server.accepted(), 2);
    wait_closed(&closed).await;
}

#[tokio::test]
async fn given_a_send_dropped_before_the_headers_then_the_connection_is_closed() {
    let closed = Arc::new(Notify::new());
    let server = RawServer::start({
        let closed = Arc::clone(&closed);
        move |mut stream, _| {
            let closed = Arc::clone(&closed);
            async move {
                let _ = read_request(&mut stream).await;
                wait_for_close(&mut stream).await;
                closed.notify_one();
            }
        }
    })
    .await;
    let client = client();
    let pending = client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer);
    let outcome = tokio::time::timeout(Duration::from_millis(200), pending).await;
    assert!(outcome.is_err());
    wait_closed(&closed).await;
}

async fn wait_for_alive_tasks(at_most: usize) {
    let metrics = tokio::runtime::Handle::current().metrics();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while metrics.num_alive_tasks() > at_most {
        assert!(
            tokio::time::Instant::now() < deadline,
            "tasks left running: {}",
            metrics.num_alive_tasks()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn given_a_response_and_a_send_dropped_then_no_task_of_the_library_is_left_running() {
    let closed = Arc::new(Notify::new());
    let server = stalling_server(Arc::clone(&closed)).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    let baseline = tokio::runtime::Handle::current()
        .metrics()
        .num_alive_tasks();
    let client = client();
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    response.body_mut().next_chunk().await.expect("chunk");
    drop(response);
    wait_closed(&closed).await;
    let pending = client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer);
    let _ = tokio::time::timeout(Duration::from_millis(100), pending).await;
    wait_for_alive_tasks(baseline).await;
}

#[tokio::test]
async fn given_a_failed_send_then_no_task_of_the_library_is_left_running() {
    let baseline = tokio::runtime::Handle::current()
        .metrics()
        .num_alive_tasks();
    let client = client();
    let endpoint = crate::support::closed_port_endpoint(false);
    let error = guarded(client.send(post(&endpoint, "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Connect { .. }));
    wait_for_alive_tasks(baseline).await;
}

#[tokio::test]
async fn given_the_last_client_dropped_then_pooled_connections_close_and_their_tasks_end() {
    let closed = Arc::new(Notify::new());
    let server = RawServer::start({
        let closed = Arc::clone(&closed);
        move |mut stream, _| {
            let closed = Arc::clone(&closed);
            async move {
                while read_request(&mut stream).await.is_ok() {
                    if write_response(&mut stream, "200 OK", &[], b"ok")
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                closed.notify_one();
            }
        }
    })
    .await;
    let baseline = tokio::runtime::Handle::current()
        .metrics()
        .num_alive_tasks();
    let client = client();
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    response.body_mut().collect(10).await.expect("body");
    drop(response);
    drop(client);
    wait_closed(&closed).await;
    wait_for_alive_tasks(baseline).await;
}
