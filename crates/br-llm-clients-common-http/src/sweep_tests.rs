use std::time::Duration;

use crate::authorizers::NoAuthorizer;
use crate::client::HttpClient;
use crate::endpoint::Endpoint;
use crate::support::frames::response_head;
use crate::support::pki::Pki;
use crate::support::raw::{RawServer, read_request, wait_for_close, write_response};
use crate::support::raw_h2::{Answer, Policy, RawH2};
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
async fn given_a_host_with_a_connection_still_in_use_then_it_stays_in_the_pool_when_another_one_expires()
 {
    let server = RawH2::start(|_| Policy {
        answer: Answer::Frames(|stream| response_head(stream, None, false)),
        ..Policy::healthy()
    })
    .await;
    let client = client();
    let endpoint = server.endpoint();
    let held = guarded(client.send(post(&endpoint, "x"), &NoAuthorizer))
        .await
        .expect("response");
    tokio::time::sleep(IDLE * 3).await;
    assert_eq!(client.pool().lock().len(), 1);
    drop(held);
}
