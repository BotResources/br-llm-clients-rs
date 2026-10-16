use std::time::Duration;

use crate::authorizers::NoAuthorizer;
use crate::client::HttpClient;
use crate::endpoint::Endpoint;
use crate::send_error::SendError;
use crate::support::frames::response_head;
use crate::support::pki::Pki;
use crate::support::raw::{RawServer, read_request, wait_for_close, write_response};
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};
use crate::support::{config, guarded, post};
use crate::tuning::Tuning;

const IDLE: Duration = Duration::from_millis(300);

fn client() -> HttpClient {
    let mut knobs = config();
    knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
    let tuning = Tuning {
        idle_limit: IDLE,
        ..Tuning::FIXED
    };
    HttpClient::with_tuning(knobs.build(), tuning).expect("client")
}

async fn hosts_forgotten(client: &HttpClient) {
    let started = tokio::time::Instant::now();
    while !client.pool().lock().is_empty() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{} hosts are still in the pool",
            client.pool().lock().len()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn one_send(client: &HttpClient, endpoint: &Endpoint) {
    let mut response = guarded(client.send(post(endpoint, "x"), &NoAuthorizer))
        .await
        .expect("response");
    response.body_mut().collect(10).await.expect("body");
}

#[tokio::test]
async fn given_an_idle_http1_connection_that_expires_then_its_host_leaves_the_pool_without_another_send()
 {
    let server = RawServer::start(|mut stream, _| async move {
        while read_request(&mut stream).await.is_ok() {
            if write_response(&mut stream, "200 OK", &[], b"ok")
                .await
                .is_err()
            {
                break;
            }
        }
        wait_for_close(&mut stream).await;
    })
    .await;
    let client = client();
    one_send(&client, &server.endpoint("/")).await;
    assert_eq!(client.pool().lock().len(), 1);
    hosts_forgotten(&client).await;
}

#[tokio::test]
async fn given_an_idle_http2_connection_that_expires_then_its_host_leaves_the_pool_without_another_send()
 {
    let server = RawH2::start(|_| Policy::healthy()).await;
    let client = client();
    one_send(&client, &server.endpoint()).await;
    assert_eq!(client.pool().lock().len(), 1);
    hosts_forgotten(&client).await;
}

#[tokio::test]
async fn given_two_connections_to_a_host_when_the_idle_one_expires_then_the_host_stays_for_the_one_in_use()
 {
    let server = RawH2::start(|index| Policy {
        announce: Announce::Limit(if index == 0 { 1 } else { 100 }),
        answer: if index == 0 {
            Answer::Frames(|stream| response_head(stream, None, false))
        } else {
            Answer::After(Duration::ZERO)
        },
        ..Policy::healthy()
    })
    .await;
    let client = client();
    let endpoint = server.endpoint();
    let held = guarded(client.send(post(&endpoint, "held"), &NoAuthorizer))
        .await
        .expect("response");
    one_send(&client, &endpoint).await;
    assert_eq!(server.accepted(), 2);
    assert_eq!(client.pool().h2_connections(), 2);
    let started = tokio::time::Instant::now();
    while client.pool().h2_connections() > 1 {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the idle connection never expired"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(IDLE * 2).await;
    assert_eq!(client.pool().lock().len(), 1);
    assert_eq!(client.pool().h2_connections(), 1);
    drop(held);
    hosts_forgotten(&client).await;
}

#[tokio::test]
async fn given_connections_that_announced_a_limit_of_zero_then_each_is_closed_after_the_idle_limit_without_a_stream()
 {
    let server = RawH2::start(|_| Policy::announcing(Announce::Limit(0))).await;
    let client = client();
    let error = guarded(client.send(post(&server.endpoint(), "x"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Connect { .. }), "{error:?}");
    assert_eq!(server.accepted(), 3);
    let started = tokio::time::Instant::now();
    while client.pool().h2_connections() > 0 || server.closed() < 3 {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{} connections are pooled and {} are closed after the idle limit",
            client.pool().h2_connections(),
            server.closed()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    hosts_forgotten(&client).await;
}
