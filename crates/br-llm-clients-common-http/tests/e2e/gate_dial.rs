use std::time::Duration;

use br_llm_clients_common_http::{NoAuthorizer, SendError};

use crate::support::gate_kit::{answering, options, run, trusting, until_accepted};
use crate::support::h2_server::TlsServer;
use crate::support::pki::Pki;
use crate::support::{guarded, post};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_dial_that_fails_then_every_send_that_waited_for_it_fails_with_it_and_none_dials_again()
 {
    let server = TlsServer::start_with(
        Pki::other_ca().acceptor(&[b"h2"]),
        answering(Duration::ZERO),
        options(100, Duration::from_millis(400), 0),
    )
    .await;
    let client = trusting(Duration::from_secs(20), Duration::from_secs(5));
    let outcomes = guarded(run(&client, &server, 30)).await;
    for outcome in &outcomes {
        assert!(matches!(outcome, Err(SendError::Tls { .. })), "{outcome:?}");
    }
    assert_eq!(server.accepted(), 1);
}

#[tokio::test]
async fn given_a_failed_dial_then_the_next_send_dials_again_and_succeeds() {
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        answering(Duration::ZERO),
        options(100, Duration::ZERO, 1),
    )
    .await;
    let client = trusting(Duration::from_millis(400), Duration::from_secs(5));
    let first = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(
        matches!(first, SendError::ConnectTimeout { .. }),
        "{first:?}"
    );
    let mut second = guarded(client.send(post(&server.endpoint("/"), "b"), &NoAuthorizer))
        .await
        .expect("second send");
    assert_eq!(
        second.body_mut().collect(10).await.expect("body").bytes,
        b"ok"
    );
    assert_eq!(server.accepted(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_the_dialing_send_dropped_mid_dial_then_the_sends_that_waited_complete_on_another_dial()
 {
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        answering(Duration::ZERO),
        options(100, Duration::ZERO, 1),
    )
    .await;
    let client = trusting(Duration::from_secs(30), Duration::from_secs(5));
    let holder = {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        tokio::spawn(async move {
            client
                .send(post(&endpoint, "holder"), &NoAuthorizer)
                .await
                .map(|_| ())
        })
    };
    until_accepted(&server, 1).await;
    let mut waiters = tokio::task::JoinSet::new();
    for _ in 0..5 {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        waiters.spawn(async move {
            client
                .send(post(&endpoint, "waiter"), &NoAuthorizer)
                .await
                .map(|_| ())
        });
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
    holder.abort();
    let finished = tokio::time::timeout(Duration::from_secs(8), async {
        let mut ok = 0;
        while let Some(outcome) = waiters.join_next().await {
            if outcome.expect("join").is_ok() {
                ok += 1;
            }
        }
        ok
    })
    .await
    .expect("the waiters were left waiting for a dial that no one makes");
    assert_eq!(finished, 5);
    assert_eq!(server.accepted(), 2);
}
