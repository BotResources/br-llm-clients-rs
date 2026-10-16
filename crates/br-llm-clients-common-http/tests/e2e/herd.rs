use std::time::Duration;

use br_llm_clients_common_http::{Endpoint, HttpClient, NoAuthorizer, SendError};
use hyper::Response;

use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{config, guarded, post};

fn client_trusting(headers_timeout: Duration) -> HttpClient {
    let mut config = config();
    config.headers_timeout = headers_timeout;
    config.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
    config.client()
}

async fn quick_server(alpn: &[&[u8]], max_streams: Option<u32>) -> TlsServer {
    TlsServer::start_limited(
        Pki::trusted().acceptor(alpn),
        service(|parts| async move {
            let hold = parts.headers.contains_key("x-hold");
            let steps = if hold {
                vec![Step::Data(b"x"), Step::Stall]
            } else {
                vec![Step::Data(b"ok")]
            };
            Ok(Response::new(ScriptBody::new(steps)))
        }),
        max_streams,
        Duration::ZERO,
    )
    .await
}

async fn burst(client: &HttpClient, endpoint: &Endpoint, count: usize) -> usize {
    let mut tasks = tokio::task::JoinSet::new();
    for i in 0..count {
        let (client, endpoint) = (client.clone(), endpoint.clone());
        tasks.spawn(async move {
            let mut response = client
                .send(post(&endpoint, &format!("r{i}")), &NoAuthorizer)
                .await
                .expect("response");
            response.body_mut().collect(10).await.expect("body").bytes
        });
    }
    let mut succeeded = 0;
    while let Some(bytes) = tasks.join_next().await {
        assert_eq!(bytes.expect("join"), b"ok");
        succeeded += 1;
    }
    succeeded
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_16_and_200_concurrent_first_sends_on_a_fresh_client_then_one_http2_connection_is_opened()
 {
    for count in [16usize, 200] {
        let server = quick_server(&[b"h2"], Some(1000)).await;
        let client = client_trusting(Duration::from_secs(20));
        let done = guarded(burst(&client, &server.endpoint("/"), count)).await;
        assert_eq!(done, count);
        assert_eq!(server.accepted(), 1, "{count} concurrent first sends");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_more_than_sixteen_concurrent_first_sends_to_an_http1_host_then_each_opens_its_own_connection()
 {
    const SENDS: usize = 40;
    let server = TlsServer::start_limited(
        Pki::trusted().acceptor(&[b"http/1.1"]),
        service(|_| async {
            tokio::time::sleep(Duration::from_millis(700)).await;
            Ok(Response::new(ScriptBody::new([Step::Data(b"ok")])))
        }),
        None,
        Duration::ZERO,
    )
    .await;
    let client = client_trusting(Duration::from_secs(20));
    assert_eq!(
        guarded(burst(&client, &server.endpoint("/"), SENDS)).await,
        SENDS
    );
    assert!(
        server.accepted() > 16,
        "{} connections for {SENDS} concurrent sends",
        server.accepted()
    );
}

#[tokio::test]
async fn given_more_streams_than_the_server_allows_then_another_connection_is_opened() {
    let server = quick_server(&[b"h2"], Some(2)).await;
    let client = client_trusting(Duration::from_secs(20));
    let mut held = Vec::new();
    for _ in 0..5 {
        let request = post(&server.endpoint("/"), "held")
            .with_header("x-hold", "1")
            .expect("header");
        held.push(
            guarded(client.send(request, &NoAuthorizer))
                .await
                .expect("held"),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(server.accepted(), 3);
    drop(held);
}

#[tokio::test]
async fn given_all_sixteen_connections_full_then_a_send_waits_and_fails_within_the_headers_timeout()
{
    let server = quick_server(&[b"h2"], Some(2)).await;
    let client = client_trusting(Duration::from_millis(1500));
    let mut held = Vec::new();
    for _ in 0..32 {
        let request = post(&server.endpoint("/"), "held")
            .with_header("x-hold", "1")
            .expect("header");
        held.push(
            guarded(client.send(request, &NoAuthorizer))
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "held {} after {} conns: {e:?}",
                        held.len(),
                        server.accepted()
                    )
                }),
        );
    }
    let started = std::time::Instant::now();
    let error = guarded(client.send(post(&server.endpoint("/"), "17th"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(
        matches!(error, SendError::HeadersTimeout { .. }),
        "{error:?}"
    );
    assert!(started.elapsed() >= Duration::from_millis(1400));
    assert_eq!(server.accepted(), 16);
    drop(held);
}

#[tokio::test]
async fn given_a_stream_freed_while_a_send_waits_then_the_send_uses_it() {
    let server = quick_server(&[b"h2"], Some(2)).await;
    let client = client_trusting(Duration::from_secs(20));
    let mut held = Vec::new();
    for _ in 0..32 {
        let request = post(&server.endpoint("/"), "held")
            .with_header("x-hold", "1")
            .expect("header");
        held.push(
            guarded(client.send(request, &NoAuthorizer))
                .await
                .expect("held"),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let waiting = {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        tokio::spawn(async move { client.send(post(&endpoint, "17th"), &NoAuthorizer).await })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(held.pop());
    let mut response = guarded(waiting).await.expect("join").expect("response");
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        b"ok"
    );
    assert_eq!(server.accepted(), 16);
}
