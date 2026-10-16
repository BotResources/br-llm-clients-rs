use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::Poll;
use std::time::Duration;

use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::io::{DuplexStream, duplex};

use crate::authorizers::NoAuthorizer;
use crate::client::HttpClient;
use crate::idle::IdleClock;
use crate::payload::Payload;
use crate::pool::PoolKey;
use crate::send_error::SendError;
use crate::support::raw::{RawServer, read_request, write_response};
use crate::support::{config, guarded, post};
use crate::transport::H1Conn;
use crate::unsent::{Unsent, send_h1};

type Connection = http1::Connection<TokioIo<DuplexStream>, Payload>;

struct Pooled {
    connection: Pin<Box<Connection>>,
    sender: http1::SendRequest<Payload>,
    server_side: DuplexStream,
}

async fn idle_connection() -> Pooled {
    let (client_io, server_side) = duplex(64 * 1024);
    let (sender, connection) = http1::Builder::new()
        .handshake::<_, Payload>(TokioIo::new(client_io))
        .await
        .expect("handshake");
    let mut connection = Box::pin(connection);
    poll_fn(|cx| {
        assert!(connection.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    Pooled {
        connection,
        sender,
        server_side,
    }
}

fn request() -> http::Request<Payload> {
    let mut request = http::Request::new(Payload::new(
        bytes::Bytes::from_static(b"payload"),
        Arc::new(AtomicBool::new(false)),
    ));
    *request.uri_mut() = "/x".parse().expect("uri");
    *request.method_mut() = http::Method::POST;
    request
}

fn pooled_conn(sender: http1::SendRequest<Payload>) -> H1Conn {
    let (idle, _) = IdleClock::new(Duration::from_secs(30));
    let driver = tokio::spawn(async {});
    H1Conn::new(sender, idle, driver.abort_handle())
}

#[tokio::test]
async fn given_a_connection_that_ended_when_a_request_is_handed_to_it_then_it_is_unprocessed_and_retired()
 {
    let Pooled {
        connection,
        sender,
        server_side,
    } = idle_connection().await;
    drop(server_side);
    let _ = connection.await;
    let mut conn = pooled_conn(sender);
    let taken = Arc::new(AtomicBool::new(false));
    let outcome = send_h1(&mut conn, request(), &taken).await;
    assert!(matches!(
        outcome,
        Err(Unsent::Unprocessed {
            taken: false,
            retire: true
        })
    ));
}

#[tokio::test]
async fn given_a_pooled_http1_connection_that_ends_before_it_writes_the_request_then_the_request_is_sent_once_on_a_new_connection()
 {
    let served = Arc::new(AtomicUsize::new(0));
    let server = RawServer::start({
        let served = Arc::clone(&served);
        move |mut stream, _| {
            let served = Arc::clone(&served);
            async move {
                while let Ok(received) = read_request(&mut stream).await {
                    assert_eq!(received.body, b"payload");
                    served.fetch_add(1, Ordering::SeqCst);
                    if write_response(&mut stream, "200 OK", &[], b"ok")
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
    })
    .await;
    let endpoint = server.endpoint("/x");
    let client = HttpClient::new(config().build()).expect("client");
    let Pooled {
        connection,
        sender,
        server_side,
    } = idle_connection().await;
    client
        .pool()
        .release_h1(&PoolKey::of(&endpoint), pooled_conn(sender));
    drop(server_side);
    let sending = client.send(post(&endpoint, "payload"), &NoAuthorizer);
    let ending = async move {
        let mut connection = pin!(connection);
        let _ = connection.as_mut().await;
    };
    let (sent, ()) = guarded(async { tokio::join!(sending, ending) }).await;
    let mut response = sent.unwrap_or_else(|error: SendError| panic!("{error:?}"));
    assert_eq!(
        response.body_mut().collect(16).await.expect("body").bytes,
        b"ok"
    );
    assert_eq!(served.load(Ordering::SeqCst), 1);
    assert_eq!(server.accepted(), 1);
}
