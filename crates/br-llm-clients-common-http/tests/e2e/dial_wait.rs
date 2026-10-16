use std::time::{Duration, Instant};

use br_llm_clients_common_http::{NoAuthorizer, SendError};

use crate::support::burst::{kinds, timed_burst};
use crate::support::config;
use crate::support::gate_kit::trusting;
use crate::support::pki::Pki;
use crate::support::post;
use crate::support::raw_h2::{Policy, RawH2, TlsFault};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
const BURSTS: u64 = 120;
const SIMULTANEOUS: u64 = 4;
const SENDS: usize = 50;

async fn one_burst(delay: Duration) -> (Vec<&'static str>, usize) {
    let server = RawH2::start(move |_| Policy {
        accept_delay: delay,
        ..Policy::healthy()
    })
    .await;
    let client = trusting(CONNECT_TIMEOUT, Duration::from_secs(20));
    let outcomes = timed_burst(&client, &server, SENDS).await;
    (kinds(&outcomes), server.accepted())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_dial_that_ends_just_under_the_connect_timeout_then_every_send_that_waited_for_it_has_its_outcome()
 {
    let mut mixed = Vec::new();
    for batch in 0..BURSTS / SIMULTANEOUS {
        let mut bursts = tokio::task::JoinSet::new();
        for offset in 0..SIMULTANEOUS {
            let delay = Duration::from_millis(294 + (batch * SIMULTANEOUS + offset) % 6);
            bursts.spawn(one_burst(delay));
        }
        while let Some(done) = bursts.join_next().await {
            let (kinds, accepted) = done.expect("join");
            if kinds.len() != 1 {
                mixed.push(format!("{kinds:?} on {accepted} connections"));
            }
        }
    }
    assert!(mixed.is_empty(), "{} mixed bursts: {mixed:?}", mixed.len());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_sends_waiting_behind_a_dial_that_never_ends_then_the_total_timeout_ends_every_one_of_them()
 {
    let server = RawH2::start(|_| Policy {
        tls_fault: TlsFault::Hang,
        ..Policy::healthy()
    })
    .await;
    let mut knobs = config();
    knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
    knobs.total_timeout = Some(Duration::from_millis(500));
    let client = knobs.client();
    let started = Instant::now();
    let mut sends = tokio::task::JoinSet::new();
    for _ in 0..SENDS {
        let (client, endpoint) = (client.clone(), server.endpoint());
        sends.spawn(async move { client.send(post(&endpoint, "x"), &NoAuthorizer).await });
    }
    while let Some(done) = sends.join_next().await {
        let outcome = done.expect("join");
        assert!(
            matches!(outcome, Err(SendError::TotalTimeout { .. })),
            "{outcome:?}"
        );
    }
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(server.accepted(), 1);
}
