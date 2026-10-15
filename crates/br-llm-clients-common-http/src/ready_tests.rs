use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use bytes::Bytes;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

use crate::acquire::READY_GRACE;
use crate::authorizers::NoAuthorizer;
use crate::client::HttpClient;
use crate::endpoint::Endpoint;
use crate::idle::IdleClock;
use crate::payload::Payload;
use crate::pool::PoolKey;
use crate::support::raw::{read_request, wait_for_close};
use crate::support::{config, guarded, post};
use crate::transport::H1Conn;

#[tokio::test]
async fn given_a_pooled_http1_connection_whose_readiness_stalls_then_it_is_abandoned_after_the_grace_and_a_new_one_is_used()
 {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let accepted = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    let (count, served) = (Arc::clone(&accepted), Arc::clone(&requests));
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            count.fetch_add(1, Ordering::SeqCst);
            let served = Arc::clone(&served);
            tokio::spawn(async move {
                while read_request(&mut stream).await.is_ok() {
                    served.fetch_add(1, Ordering::SeqCst);
                    let reply = b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok";
                    if stream.write_all(reply).await.is_err() {
                        return;
                    }
                }
                wait_for_close(&mut stream).await;
            });
        }
    });
    let endpoint = Endpoint::plain_http(&format!("http://{addr}/x")).expect("endpoint");
    let client = HttpClient::new(config().build()).expect("client");
    let stream = TcpStream::connect(addr).await.expect("connect");
    let (sender, connection) = http1::Builder::new()
        .handshake::<_, Payload>(TokioIo::new(stream))
        .await
        .expect("handshake");
    let never_polled = tokio::spawn(async move {
        let _connection = connection;
        std::future::pending::<()>().await;
    });
    let (idle, _) = IdleClock::new(Duration::from_secs(30));
    let stalled = H1Conn::new(sender, idle, never_polled.abort_handle());
    client.pool().release_h1(&PoolKey::of(&endpoint), stalled);

    let started = Instant::now();
    let mut response = guarded(client.send(post(&endpoint, "x"), &NoAuthorizer))
        .await
        .expect("response");
    let elapsed = started.elapsed();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        Bytes::from_static(b"ok")
    );
    assert!(elapsed >= READY_GRACE, "{elapsed:?}");
    assert!(
        elapsed < READY_GRACE + Duration::from_secs(2),
        "{elapsed:?}"
    );
    assert_eq!(accepted.load(Ordering::SeqCst), 2);
    assert_eq!(requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn given_a_pooled_http1_connection_whose_idle_expiry_was_claimed_then_it_is_not_used_even_if_it_is_ready()
 {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let accepted = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&accepted);
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            count.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                while read_request(&mut stream).await.is_ok() {
                    let reply = b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok";
                    if stream.write_all(reply).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    let endpoint = Endpoint::plain_http(&format!("http://{addr}/x")).expect("endpoint");
    let client = HttpClient::new(config().build()).expect("client");
    let stream = TcpStream::connect(addr).await.expect("connect");
    let (sender, connection) = http1::Builder::new()
        .handshake::<_, Payload>(TokioIo::new(stream))
        .await
        .expect("handshake");
    let driver = tokio::spawn(connection);
    let (idle, mut watch) = IdleClock::new(Duration::from_secs(30));
    idle.arm();
    let armed = watch.armed_until().expect("armed");
    assert!(watch.expire(armed));
    let expired = H1Conn::new(sender, idle, driver.abort_handle());
    client.pool().release_h1(&PoolKey::of(&endpoint), expired);

    let mut response = guarded(client.send(post(&endpoint, "x"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        Bytes::from_static(b"ok")
    );
    assert_eq!(accepted.load(Ordering::SeqCst), 2);
}
