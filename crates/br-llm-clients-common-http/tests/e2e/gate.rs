use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use br_llm_clients_common_http::{NoAuthorizer, SendError};
use hyper::Response;
use tokio::sync::Semaphore;

use crate::support::gate_kit::{answering, options, run, trusting};
use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{guarded, post};

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
    let received = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(Semaphore::new(0));
    let service = {
        let (received, release) = (Arc::clone(&received), Arc::clone(&release));
        service(move |parts| {
            let (received, release) = (Arc::clone(&received), Arc::clone(&release));
            async move {
                received.fetch_add(1, Ordering::SeqCst);
                if parts.headers.contains_key("x-never") {
                    std::future::pending::<()>().await;
                }
                let _ = release.acquire().await;
                Ok(Response::new(ScriptBody::new([Step::Data(b"ok")])))
            }
        })
    };
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        service,
        options(1, Duration::ZERO, 0),
    )
    .await;
    let headers_timeout = Duration::from_millis(2000);
    let client = trusting(Duration::from_secs(20), headers_timeout);
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
    let waiting_since = Instant::now();
    while received.load(Ordering::SeqCst) < 16 {
        assert!(
            waiting_since.elapsed() < Duration::from_secs(10),
            "only {} of 16 holders reached the server",
            received.load(Ordering::SeqCst)
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let started = Instant::now();
    let seventeenth = {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        tokio::spawn(async move {
            let request = post(&endpoint, "17th")
                .with_header("x-never", "1")
                .expect("header");
            client.send(request, &NoAuthorizer).await
        })
    };
    tokio::time::sleep(headers_timeout * 6 / 10).await;
    release.add_permits(16);
    let error = guarded(seventeenth)
        .await
        .expect("join")
        .map(|_| ())
        .unwrap_err();
    let elapsed = started.elapsed();
    assert!(
        matches!(error, SendError::HeadersTimeout { .. }),
        "{error:?}"
    );
    assert!(elapsed >= headers_timeout * 9 / 10, "{elapsed:?}");
    assert!(elapsed < headers_timeout * 14 / 10, "{elapsed:?}");
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
    let arrived = Arc::new(Semaphore::new(0));
    let together = Arc::new(tokio::sync::Barrier::new(3));
    let service = {
        let (arrived, together) = (Arc::clone(&arrived), Arc::clone(&together));
        service(move |_| {
            let (arrived, together) = (Arc::clone(&arrived), Arc::clone(&together));
            async move {
                arrived.add_permits(1);
                together.wait().await;
                Ok(Response::new(ScriptBody::new([Step::Data(b"ok")])))
            }
        })
    };
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        service,
        options(1, Duration::ZERO, 0),
    )
    .await;
    let client = trusting(Duration::from_secs(20), Duration::from_secs(10));
    let outcomes = guarded(run(&client, &server, 3)).await;
    assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
    assert_eq!(server.accepted(), 3);
    assert_eq!(arrived.available_permits(), 3);
}

#[tokio::test]
async fn given_a_send_that_waited_for_a_stream_and_dials_again_then_headers_timeout_restarts_when_the_new_connection_is_ready()
 {
    let received = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(Semaphore::new(0));
    let service = {
        let (received, release) = (Arc::clone(&received), Arc::clone(&release));
        service(move |_| {
            let (received, release) = (Arc::clone(&received), Arc::clone(&release));
            async move {
                received.fetch_add(1, Ordering::SeqCst);
                let _ = release.acquire().await;
                Ok(Response::new(ScriptBody::new([Step::Data(b"ok")])))
            }
        })
    };
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        service,
        options(1, Duration::ZERO, 0),
    )
    .await;
    let headers_timeout = Duration::from_millis(1500);
    let client = trusting(Duration::from_secs(20), headers_timeout);
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
    let waiting_since = Instant::now();
    while received.load(Ordering::SeqCst) < 16 {
        assert!(
            waiting_since.elapsed() < Duration::from_secs(10),
            "only {} of 16 holders reached the server",
            received.load(Ordering::SeqCst)
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let waiter = {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        tokio::spawn(async move { client.send(post(&endpoint, "waiter"), &NoAuthorizer).await })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;
    server.set_handshake_delay(headers_timeout + Duration::from_millis(1000));
    release.add_permits(100);
    server.kill_connections();
    let mut response = guarded(waiter)
        .await
        .expect("join")
        .expect("the new connection is ready within a new headers deadline");
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        b"ok"
    );
    while holders.join_next().await.is_some() {}
}
