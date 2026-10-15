use std::collections::HashMap;

use br_llm_clients_common_http::{Request, SendError};

use crate::support::pki::Pki;
use crate::support::probe::{Probe, Script};
use crate::support::spy::Spy;
use crate::support::{guarded, trusting_client};

async fn rounds(script: Script, rounds: usize, per_round: usize) -> (usize, usize) {
    let mut sent = 0usize;
    let mut duplicated = 0usize;
    for round in 0..rounds {
        let probe = Probe::start(script).await;
        let client = trusting_client(&Pki::trusted().ca_pem);
        let mut tasks = tokio::task::JoinSet::new();
        for i in 0..per_round {
            let (client, endpoint) = (client.clone(), probe.endpoint());
            let id = format!("{round}-{i}");
            tasks.spawn(async move {
                let request = Request::new(endpoint, "payload")
                    .with_header("x-id", &id)
                    .expect("header");
                client.send(request, &Spy::returning(&[])).await.map(|_| ())
            });
            sent += 1;
        }
        while tasks.join_next().await.is_some() {}
        let mut seen: HashMap<String, usize> = HashMap::new();
        for id in probe.arrivals() {
            *seen.entry(id).or_default() += 1;
        }
        duplicated += seen.values().filter(|count| **count > 1).count();
    }
    (sent, duplicated)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_server_that_closes_after_reading_the_head_then_no_request_arrives_twice() {
    let (sent, duplicated) = rounds(Script::CloseAfterReceiving, 200, 16).await;
    assert_eq!(
        duplicated, 0,
        "{duplicated} of {sent} requests arrived twice"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_server_with_a_zero_window_that_closes_after_reading_heads_then_no_request_arrives_twice()
 {
    let (sent, duplicated) = rounds(Script::ZeroWindowClose, 100, 32).await;
    assert_eq!(
        duplicated, 0,
        "{duplicated} of {sent} requests arrived twice"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_goaway_that_keeps_the_stream_and_closes_then_no_request_arrives_twice() {
    let (sent, duplicated) = rounds(Script::GoAwayAfterReceiving, 200, 16).await;
    assert_eq!(
        duplicated, 0,
        "{duplicated} of {sent} requests arrived twice"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_graceful_shutdowns_after_few_requests_under_load_then_no_request_arrives_twice_smoke()
 {
    for after in [2usize, 10, 40] {
        let (sent, duplicated) = rounds(Script::Graceful(after), 60, 64).await;
        assert_eq!(
            duplicated, 0,
            "graceful shutdown after {after} requests: {duplicated} of {sent} requests arrived twice"
        );
    }
}

async fn single_send_against(script: Script) -> (Result<(), SendError>, Probe) {
    let probe = Probe::start(script).await;
    let request = Request::new(probe.endpoint(), "payload")
        .with_header("x-id", "r1")
        .expect("header");
    let client = trusting_client(&Pki::trusted().ca_pem);
    let outcome = guarded(client.send(request, &Spy::returning(&[])))
        .await
        .map(|_| ());
    (outcome, probe)
}

#[tokio::test]
async fn given_a_server_with_a_zero_window_that_closes_after_the_head_then_the_request_is_not_sent_again()
 {
    let (outcome, probe) = single_send_against(Script::ZeroWindowClose).await;
    assert!(
        matches!(outcome, Err(SendError::Interrupted { .. })),
        "{outcome:?}"
    );
    assert_eq!(probe.arrivals(), ["r1"]);
    assert_eq!(probe.accepted.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn given_a_server_with_a_zero_window_that_sends_a_goaway_and_closes_after_the_head_then_the_request_is_not_sent_again()
 {
    let (outcome, probe) = single_send_against(Script::ZeroWindowGoAway).await;
    assert!(
        matches!(outcome, Err(SendError::Interrupted { .. })),
        "{outcome:?}"
    );
    assert_eq!(probe.arrivals(), ["r1"]);
    assert_eq!(probe.accepted.load(std::sync::atomic::Ordering::SeqCst), 1);
}
