use br_llm_clients_common_http::NoAuthorizer;
use tokio::io::AsyncWriteExt;

use crate::support::canned::serve_response;
use crate::support::raw::{RawServer, read_request, write_raw, write_response};
use crate::support::{client, guarded, post};

async fn keep_alive_server() -> RawServer {
    RawServer::start(|mut stream, _| async move {
        while read_request(&mut stream).await.is_ok() {
            if write_response(&mut stream, "200 OK", &[], b"ok")
                .await
                .is_err()
            {
                break;
            }
        }
    })
    .await
}

#[tokio::test]
async fn given_two_sequential_requests_when_the_first_body_is_read_then_the_connection_is_reused() {
    let server = keep_alive_server().await;
    let client = client();
    for _ in 0..3 {
        let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
            .await
            .expect("response");
        assert_eq!(
            response.body_mut().collect(10).await.expect("body").bytes,
            b"ok"
        );
    }
    assert_eq!(server.accepted(), 1);
}

#[tokio::test]
async fn given_a_clone_when_sending_then_it_shares_the_connections() {
    let server = keep_alive_server().await;
    let first = client();
    let second = first.clone();
    let mut response = guarded(first.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    response.body_mut().collect(10).await.expect("body");
    let mut response = guarded(second.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    response.body_mut().collect(10).await.expect("body");
    assert_eq!(server.accepted(), 1);
}

#[tokio::test]
async fn given_a_response_without_a_body_that_is_never_read_then_the_connection_is_still_reused() {
    let server = RawServer::start(|mut stream, _| async move {
        while read_request(&mut stream).await.is_ok() {
            if write_raw(&mut stream, b"HTTP/1.1 204 No Content\r\n\r\n")
                .await
                .is_err()
            {
                break;
            }
        }
    })
    .await;
    let client = client();
    for _ in 0..3 {
        let response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
            .await
            .expect("response");
        assert_eq!(response.status(), 204);
        drop(response);
    }
    assert_eq!(server.accepted(), 1);
}

#[tokio::test]
async fn given_connection_close_when_the_response_ends_then_the_next_request_uses_a_new_connection()
{
    let server = serve_response("200 OK", &[], b"ok").await;
    let client = client();
    for _ in 0..2 {
        let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
            .await
            .expect("response");
        response.body_mut().collect(10).await.expect("body");
    }
    assert_eq!(server.accepted(), 2);
}

#[tokio::test]
async fn given_a_pooled_connection_closed_by_the_server_when_sending_then_a_new_one_is_used_without_error()
 {
    let server = RawServer::start(|mut stream, _| async move {
        if read_request(&mut stream).await.is_ok() {
            let _ = write_response(&mut stream, "200 OK", &[], b"ok").await;
        }
        let _ = stream.shutdown().await;
    })
    .await;
    let client = client();
    for _ in 0..4 {
        let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
            .await
            .expect("response");
        assert_eq!(
            response.body_mut().collect(10).await.expect("body").bytes,
            b"ok"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(server.accepted(), 4);
}

#[tokio::test]
async fn given_a_request_written_on_a_pooled_connection_that_then_closes_then_it_is_not_sent_again()
{
    let requests = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let server = RawServer::start({
        let requests = std::sync::Arc::clone(&requests);
        move |mut stream, _| {
            let requests = std::sync::Arc::clone(&requests);
            async move {
                while read_request(&mut stream).await.is_ok() {
                    let seen = requests.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if seen == 0
                        && write_response(&mut stream, "200 OK", &[], b"ok")
                            .await
                            .is_ok()
                    {
                        continue;
                    }
                    break;
                }
            }
        }
    })
    .await;
    let client = client();
    let mut first = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    first.body_mut().collect(10).await.expect("body");
    let error = guarded(client.send(post(&server.endpoint("/"), "b"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            br_llm_clients_common_http::SendError::Interrupted { .. }
        ),
        "{error:?}"
    );
    assert_eq!(requests.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(server.accepted(), 1);
}
