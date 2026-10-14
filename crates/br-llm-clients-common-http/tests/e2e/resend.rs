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
async fn given_a_stream_excluded_by_a_goaway_when_sent_then_it_is_sent_again_on_a_new_connection_once()
 {
    let probe = Probe::start(Script::GoAwayOnFirstConnections(1)).await;
    let spy = Spy::returning(&[("x-api-key", "k")]);
    let mut response = guarded(client().send(identified(&probe, "r1"), &spy))
        .await
        .expect("sent again");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        b"ok"
    );
    assert_eq!(probe.accepted.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(probe.processed(), ["r1"]);
    assert_eq!(probe.arrivals(), ["r1"]);
    assert_eq!(spy.calls(), 1);
}

#[tokio::test]
async fn given_a_refused_stream_when_sent_then_it_is_sent_again_on_a_new_connection_once() {
    let probe = Probe::start(Script::RefuseFirst(1)).await;
    let spy = Spy::returning(&[("x-api-key", "k")]);
    let mut response = guarded(client().send(identified(&probe, "r1"), &spy))
        .await
        .expect("sent again");
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        b"ok"
    );
    assert_eq!(probe.accepted.load(std::sync::atomic::Ordering::SeqCst), 2);
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
    assert_eq!(probe.accepted.load(std::sync::atomic::Ordering::SeqCst), 3);
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
    let probe = Probe::start(Script::GoAwayOnFirstConnections(40)).await;
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
            failure.starts_with("Interrupted") || failure == "body",
            "{failure}"
        );
    }
}
