use std::time::{Duration, Instant};

use br_llm_clients_common_http::{NoAuthorizer, SendError};

use crate::support::gate_kit::{answering, options, run, trusting};
use crate::support::h2_server::TlsServer;
use crate::support::pki::Pki;
use crate::support::{guarded, post};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_burst_larger_than_the_server_limit_then_every_send_succeeds_without_queueing_in_a_connection()
 {
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        answering(Duration::from_millis(50)),
        options(100, Duration::ZERO, 0),
    )
    .await;
    let client = trusting(Duration::from_secs(20), Duration::from_secs(5));
    let outcomes = guarded(run(&client, &server, 300)).await;
    assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
    assert!(server.accepted() <= 4, "{} connections", server.accepted());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_server_with_one_stream_per_connection_then_a_burst_is_served() {
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        answering(Duration::from_millis(30)),
        options(1, Duration::ZERO, 0),
    )
    .await;
    let client = trusting(Duration::from_secs(20), Duration::from_secs(10));
    let outcomes = guarded(run(&client, &server, 60)).await;
    assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_heavy_contention_for_few_streams_then_no_wakeup_is_lost() {
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        answering(Duration::ZERO),
        options(1, Duration::ZERO, 0),
    )
    .await;
    let client = trusting(Duration::from_secs(20), Duration::from_secs(10));
    let started = Instant::now();
    let mut waves = tokio::task::JoinSet::new();
    for _ in 0..32 {
        let (client, server_endpoint) = (client.clone(), server.endpoint("/"));
        waves.spawn(async move {
            let mut failures = 0usize;
            for _ in 0..40 {
                match client
                    .send(post(&server_endpoint, "a"), &NoAuthorizer)
                    .await
                {
                    Ok(mut response) => {
                        let _ = response.body_mut().collect(10).await;
                    }
                    Err(_) => failures += 1,
                }
            }
            failures
        });
    }
    let mut failures = 0;
    while let Some(wave) = waves.join_next().await {
        failures += wave.expect("join");
    }
    assert_eq!(failures, 0);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn given_a_wait_for_a_stream_and_the_exchange_then_headers_timeout_is_one_deadline() {
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        answering(Duration::from_millis(800)),
        options(1, Duration::ZERO, 0),
    )
    .await;
    let client = trusting(Duration::from_secs(20), Duration::from_millis(1000));
    let mut holders = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        holders.spawn(async move {
            client
                .send(post(&endpoint, "a"), &NoAuthorizer)
                .await
                .map(|_| ())
        });
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    let started = Instant::now();
    let error = guarded(client.send(post(&server.endpoint("/"), "17th"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(
        matches!(error, SendError::HeadersTimeout { .. }),
        "{error:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(1400),
        "{:?}",
        started.elapsed()
    );
    while holders.join_next().await.is_some() {}
}

#[tokio::test]
async fn given_every_connection_dying_while_a_send_waits_for_a_stream_then_the_send_dials_again() {
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        answering(Duration::ZERO),
        options(2, Duration::ZERO, 0),
    )
    .await;
    let client = trusting(Duration::from_secs(20), Duration::from_secs(8));
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
    }
    assert_eq!(server.accepted(), 16);
    let waiting = {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        tokio::spawn(async move { client.send(post(&endpoint, "waiter"), &NoAuthorizer).await })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;
    server.kill_connections();
    let started = Instant::now();
    let mut response = guarded(waiting)
        .await
        .expect("join")
        .expect("the waiter dials again");
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        b"ok"
    );
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    drop(held);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_concurrent_first_sends_and_a_one_stream_server_then_none_queues_inside_a_connection()
{
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        answering(Duration::from_millis(600)),
        options(1, Duration::ZERO, 0),
    )
    .await;
    let client = trusting(Duration::from_secs(20), Duration::from_millis(1300));
    let outcomes = guarded(run(&client, &server, 3)).await;
    assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
    assert_eq!(server.accepted(), 3);
}
