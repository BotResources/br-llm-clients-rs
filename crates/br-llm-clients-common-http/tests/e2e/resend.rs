use std::sync::atomic::Ordering;

use br_llm_clients_common_http::{HttpClient, Request, SendError};

use crate::support::pki::Pki;
use crate::support::probe::{Probe, Script};
use crate::support::spy::Spy;
use crate::support::{guarded, trusting_client};

fn identified(probe: &Probe, id: &str) -> Request {
    Request::new(probe.endpoint(), "payload")
        .with_header("x-id", id)
        .expect("header")
}

fn client() -> HttpClient {
    trusting_client(&Pki::trusted().ca_pem)
}

#[tokio::test]
async fn given_a_stream_excluded_by_a_goaway_when_sent_then_it_fails_interrupted_and_is_not_sent_again()
 {
    let probe = Probe::start(Script::GoAway {
        first_connections: 1,
        last_stream: 0,
        code: 0,
    })
    .await;
    let spy = Spy::returning(&[("x-api-key", "k")]);
    let client = client();
    let error = guarded(client.send(identified(&probe, "r1"), &spy))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
    assert_eq!(spy.calls(), 1);
    let mut next = guarded(client.send(identified(&probe, "r2"), &spy))
        .await
        .expect("the next send dials a new connection");
    assert_eq!(
        next.body_mut().collect(10).await.expect("body").bytes,
        b"ok"
    );
    assert_eq!(probe.accepted.load(Ordering::SeqCst), 2);
    assert_eq!(probe.processed(), ["r2"]);
    let mut arrivals = probe.arrivals();
    for _ in 0..200 {
        if arrivals.len() >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        arrivals = probe.arrivals();
    }
    arrivals.sort();
    assert_eq!(arrivals, ["?", "r2"]);
}

#[tokio::test]
async fn given_a_refused_stream_when_sent_then_it_is_sent_again_through_the_same_healthy_connection_once()
 {
    let probe = Probe::start(Script::RefuseFirst(1)).await;
    let spy = Spy::returning(&[("x-api-key", "k")]);
    let mut response = guarded(client().send(identified(&probe, "r1"), &spy))
        .await
        .expect("sent again");
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        b"ok"
    );
    assert_eq!(probe.accepted.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(probe.arrivals(), ["r1", "r1"]);
    assert_eq!(probe.processed(), ["r1"]);
    assert_eq!(spy.calls(), 1);
}

#[tokio::test]
async fn given_a_server_that_always_refuses_when_sent_then_it_stops_after_three_attempts_with_interrupted()
 {
    let probe = Probe::start(Script::RefuseAll).await;
    let error = guarded(client().send(identified(&probe, "r1"), &Spy::returning(&[])))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
    assert_eq!(probe.arrivals().len(), 3);
    assert!(probe.processed().is_empty());
    assert_eq!(probe.accepted.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn given_a_connection_that_closes_after_the_request_arrived_then_it_is_not_sent_again() {
    let probe = Probe::start(Script::CloseAfterReceiving).await;
    let error = guarded(client().send(identified(&probe, "r1"), &Spy::returning(&[])))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
    assert_eq!(probe.arrivals(), ["r1"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_connections_that_close_gracefully_under_load_then_no_request_ever_arrives_twice() {
    let probe = Probe::start(Script::GoAway {
        first_connections: 6,
        last_stream: 0,
        code: 0,
    })
    .await;
    let client = client();
    let mut tasks = tokio::task::JoinSet::new();
    for i in 0..120 {
        let client = client.clone();
        let request = identified(&probe, &format!("r{i}"));
        tasks.spawn(async move {
            match client.send(request, &Spy::returning(&[])).await {
                Ok(mut response) => response
                    .body_mut()
                    .collect(10)
                    .await
                    .map(|_| ())
                    .map_err(|_| "body".to_owned()),
                Err(error) => Err(format!("{error:?}")),
            }
        });
    }
    let mut failures = Vec::new();
    while let Some(outcome) = tasks.join_next().await {
        if let Err(kind) = outcome.expect("join") {
            failures.push(kind);
        }
    }
    let processed = probe.processed();
    let mut unique = processed.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        processed.len(),
        unique.len(),
        "a request was processed twice"
    );
    for failure in &failures {
        assert!(
            failure.starts_with("Interrupted")
                || failure.starts_with("Connect { text: \"no connection could carry")
                || failure == "body",
            "{failure}"
        );
    }
}

fn goaway(first_connections: usize, last_stream: u32, code: u32) -> Script {
    Script::GoAway {
        first_connections,
        last_stream,
        code,
    }
}

async fn burst(client: &HttpClient, probe: &Probe, count: usize) -> Vec<Result<(), SendError>> {
    let mut tasks = tokio::task::JoinSet::new();
    for i in 0..count {
        let (client, request) = (client.clone(), identified(probe, &format!("r{i}")));
        tasks.spawn(async move {
            let mut response = client.send(request, &Spy::returning(&[])).await?;
            response
                .body_mut()
                .collect(10)
                .await
                .map(|_| ())
                .map_err(|_| SendError::Protocol {
                    text: "body".to_owned(),
                })
        });
    }
    let mut outcomes = Vec::new();
    while let Some(outcome) = tasks.join_next().await {
        outcomes.push(outcome.expect("join"));
    }
    outcomes
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_many_sends_on_a_connection_that_a_goaway_excludes_then_each_is_processed_at_most_once_and_the_connection_is_retired()
 {
    for count in [50usize, 100] {
        let probe = Probe::start(goaway(1, 0, 0)).await;
        let client = client();
        let outcomes = guarded(burst(&client, &probe, count)).await;
        let interrupted = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(SendError::Interrupted { .. })))
            .count();
        let succeeded = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
        assert!(interrupted >= 1, "{count} sends: {outcomes:?}");
        assert_eq!(interrupted + succeeded, count, "{outcomes:?}");
        let processed = probe.processed();
        let mut unique = processed.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(processed.len(), unique.len(), "{count} sends");
        assert_eq!(processed.len(), succeeded, "{count} sends");
        let after = guarded(burst(&client, &probe, 1)).await;
        assert!(after[0].is_ok(), "{after:?}");
        assert_eq!(probe.accepted.load(Ordering::SeqCst), 2, "{count} sends");
    }
}

#[tokio::test]
async fn given_a_goaway_that_keeps_the_stream_and_closes_then_the_request_is_not_sent_again() {
    let probe = Probe::start(goaway(1, 1, 0)).await;
    let error = guarded(client().send(identified(&probe, "r1"), &Spy::returning(&[])))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
    assert_eq!(probe.arrivals(), ["?"]);
    assert_eq!(probe.accepted.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn given_a_goaway_asking_for_http_1_1_then_the_send_fails_with_protocol_and_no_storm() {
    let probe = Probe::start(goaway(100, 0, 13)).await;
    let error = guarded(client().send(identified(&probe, "r1"), &Spy::returning(&[])))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
    assert_eq!(probe.accepted.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn given_a_stream_reset_by_the_server_for_another_reason_then_the_request_is_not_sent_again()
{
    for reason in [
        h2::Reason::INTERNAL_ERROR,
        h2::Reason::CANCEL,
        h2::Reason::STREAM_CLOSED,
    ] {
        let probe = Probe::start(Script::ResetAll(reason)).await;
        let error = guarded(client().send(identified(&probe, "r1"), &Spy::returning(&[])))
            .await
            .unwrap_err();
        assert!(
            matches!(error, SendError::Interrupted { .. }),
            "{reason:?}: {error:?}"
        );
        assert_eq!(probe.arrivals(), ["r1"], "{reason:?}");
        assert_eq!(probe.accepted.load(Ordering::SeqCst), 1, "{reason:?}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_occasional_refusals_then_the_healthy_connection_is_kept_and_nothing_is_processed_twice()
 {
    let probe = Probe::start(Script::RefuseEvery(20)).await;
    let outcomes = guarded(burst(&client(), &probe, 400)).await;
    assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
    assert!(
        probe.accepted.load(Ordering::SeqCst) <= 2,
        "{} connections",
        probe.accepted.load(Ordering::SeqCst)
    );
    let mut processed = probe.processed();
    processed.sort();
    processed.dedup();
    assert_eq!(processed.len(), 400);
}
