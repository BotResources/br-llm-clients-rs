use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use hyper::Response;

use crate::authorizers::NoAuthorizer;
use crate::client::HttpClient;
use crate::endpoint::Endpoint;
use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::proxy::Proxy;
use crate::support::raw::{RawServer, read_request, wait_for_close, write_response};
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{config, guarded, post};
use crate::tuning::Tuning;

const IDLE: Duration = Duration::from_secs(2);

fn tuning() -> Tuning {
    Tuning {
        idle_limit: IDLE,
        ping_interval: Duration::from_millis(300),
        ping_timeout: Duration::from_millis(200),
    }
}

fn client(trusting: bool) -> HttpClient {
    let mut config = config();
    if trusting {
        config.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
    }
    HttpClient::with_tuning(config, tuning()).expect("client")
}

async fn eventually(what: &str, check: impl Fn() -> bool) {
    let started = tokio::time::Instant::now();
    while !check() {
        assert!(started.elapsed() < Duration::from_secs(15), "never: {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn counting_server() -> (RawServer, Arc<AtomicUsize>) {
    let closed = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&closed);
    let server = RawServer::start(move |mut stream, _| {
        let counter = Arc::clone(&counter);
        async move {
            while read_request(&mut stream).await.is_ok() {
                if write_response(&mut stream, "200 OK", &[], b"ok")
                    .await
                    .is_err()
                {
                    break;
                }
            }
            wait_for_close(&mut stream).await;
            counter.fetch_add(1, Ordering::SeqCst);
        }
    })
    .await;
    (server, closed)
}

async fn finished_send(client: &HttpClient, endpoint: &Endpoint) {
    let mut response = guarded(client.send(post(endpoint, "a"), &NoAuthorizer))
        .await
        .expect("response");
    response.body_mut().collect(10).await.expect("body");
}

fn holding_service() -> crate::support::h2_server::Service {
    service(|parts| async move {
        let hold = parts.headers.contains_key("x-hold");
        let steps = if hold {
            vec![Step::Data(b"x"), Step::Stall]
        } else {
            vec![Step::Data(b"ok")]
        };
        Ok(Response::new(ScriptBody::new(steps)))
    })
}

#[tokio::test]
async fn given_a_pooled_http2_connection_that_goes_silent_then_the_next_send_uses_a_new_connection()
{
    let pki = Pki::trusted();
    let server = TlsServer::start(pki.acceptor(&[b"h2"]), holding_service()).await;
    let proxy = Proxy::start(server.addr).await;
    let endpoint =
        Endpoint::https(&format!("https://127.0.0.1:{}/", proxy.addr.port())).expect("endpoint");
    let client = client(true);
    let held = guarded(
        client.send(
            post(&endpoint, "held")
                .with_header("x-hold", "1")
                .expect("header"),
            &NoAuthorizer,
        ),
    )
    .await
    .expect("held");
    assert_eq!(proxy.accepted(), 1);
    proxy.blackhole(0);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let mut second = guarded(client.send(post(&endpoint, "second"), &NoAuthorizer))
        .await
        .expect("second send");
    assert_eq!(
        second.body_mut().collect(10).await.expect("body").bytes,
        b"ok"
    );
    assert_eq!(proxy.accepted(), 2);
    drop(held);
}

#[tokio::test]
async fn given_a_connection_that_answers_pings_then_it_is_kept_while_it_has_a_stream_in_flight() {
    let pki = Pki::trusted();
    let server = TlsServer::start(pki.acceptor(&[b"h2"]), holding_service()).await;
    let client = client(true);
    let held = guarded(
        client.send(
            post(&server.endpoint("/"), "held")
                .with_header("x-hold", "1")
                .expect("header"),
            &NoAuthorizer,
        ),
    )
    .await
    .expect("held");
    tokio::time::sleep(IDLE * 2 + Duration::from_millis(500)).await;
    assert_eq!(server.closed(), 0);
    drop(held);
}

#[tokio::test]
async fn given_an_idle_http1_connection_then_the_library_closes_it_after_the_idle_limit() {
    let (server, closed) = counting_server().await;
    let client = client(false);
    finished_send(&client, &server.endpoint("/")).await;
    tokio::time::sleep(IDLE / 4).await;
    assert_eq!(closed.load(Ordering::SeqCst), 0);
    eventually("the idle connection is closed", || {
        closed.load(Ordering::SeqCst) == 1
    })
    .await;
    finished_send(&client, &server.endpoint("/")).await;
    assert_eq!(server.accepted(), 2);
}

#[tokio::test]
async fn given_a_used_http1_connection_then_its_idle_time_restarts_at_each_use() {
    let (server, closed) = counting_server().await;
    let client = client(false);
    for _ in 0..5 {
        finished_send(&client, &server.endpoint("/")).await;
        tokio::time::sleep(IDLE / 3).await;
    }
    assert_eq!(closed.load(Ordering::SeqCst), 0);
    assert_eq!(server.accepted(), 1);
}

#[tokio::test]
async fn given_ten_connections_back_to_idle_then_only_eight_are_kept() {
    let (server, closed) = counting_server().await;
    let client = client(false);
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..10 {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        tasks.spawn(async move {
            let mut response = client
                .send(post(&endpoint, "a"), &NoAuthorizer)
                .await
                .expect("response");
            tokio::time::sleep(Duration::from_millis(300)).await;
            response.body_mut().collect(10).await.expect("body");
        });
    }
    while tasks.join_next().await.is_some() {}
    eventually("two connections beyond the cap are closed", || {
        closed.load(Ordering::SeqCst) >= 2
    })
    .await;
    assert_eq!(server.accepted(), 10);
    assert_eq!(closed.load(Ordering::SeqCst), 2);
    eventually("the rest expire", || closed.load(Ordering::SeqCst) == 10).await;
}

#[tokio::test]
async fn given_an_idle_http2_connection_then_the_library_closes_it_after_the_idle_limit() {
    let pki = Pki::trusted();
    let server = TlsServer::start(pki.acceptor(&[b"h2"]), holding_service()).await;
    let client = client(true);
    finished_send(&client, &server.endpoint("/")).await;
    tokio::time::sleep(IDLE / 4).await;
    assert_eq!(server.closed(), 0);
    eventually("the idle connection is closed", || server.closed() == 1).await;
    finished_send(&client, &server.endpoint("/")).await;
    assert_eq!(server.accepted(), 2);
}

#[tokio::test]
async fn given_an_http2_connection_with_a_stream_in_flight_then_it_is_not_closed_as_idle_until_the_stream_ends()
 {
    let pki = Pki::trusted();
    let server = TlsServer::start(pki.acceptor(&[b"h2"]), holding_service()).await;
    let client = client(true);
    let held = guarded(
        client.send(
            post(&server.endpoint("/"), "held")
                .with_header("x-hold", "1")
                .expect("header"),
            &NoAuthorizer,
        ),
    )
    .await
    .expect("held");
    tokio::time::sleep(IDLE + IDLE / 2).await;
    assert_eq!(server.closed(), 0);
    drop(held);
    eventually("the connection expires once idle", || server.closed() == 1).await;
}
