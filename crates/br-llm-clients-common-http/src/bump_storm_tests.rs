use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::task::JoinSet;

use crate::authorizers::NoAuthorizer;
use crate::client::HttpClient;
use crate::pool::PoolKey;
use crate::send_error::SendError;
use crate::support::pki::Pki;
use crate::support::raw_h2::{Announce, Answer, Ending, Policy, RawH2};
use crate::support::{client_with, post};
use crate::transport::bump;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
const SLOW_DIAL: Duration = Duration::from_millis(250);
const SLOW_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const SENDS: usize = 64;
const BUMPERS: usize = 3;
const REPETITIONS: usize = 16;

struct Round {
    accepted: usize,
    took: Duration,
    failures: Vec<SendError>,
}

fn silent() -> Policy {
    Policy {
        answer: Answer::Never,
        ..Policy::announcing(Announce::Nothing)
    }
}

fn eof_before_settings() -> Policy {
    Policy {
        ends_after: Some((Duration::from_millis(50), Ending::Eof)),
        accept_delay: SLOW_DIAL,
        ..silent()
    }
}

fn zero_then_eof() -> Policy {
    Policy {
        ends_after: Some((Duration::from_millis(50), Ending::Eof)),
        accept_delay: SLOW_DIAL,
        ..Policy::announcing(Announce::Limit(0))
    }
}

fn bumping(
    client: &HttpClient,
    key: &PoolKey,
    stop: &Arc<AtomicBool>,
) -> Vec<std::thread::JoinHandle<()>> {
    (0..BUMPERS)
        .map(|_| {
            let (client, key, stop) = (client.clone(), key.clone(), Arc::clone(stop));
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let signal = client
                        .pool()
                        .lock()
                        .get(&key)
                        .map(|host| Arc::clone(&host.signal));
                    match signal {
                        Some(signal) => bump(&signal),
                        None => std::thread::yield_now(),
                    }
                }
            })
        })
        .collect()
}

async fn round(policy: fn() -> Policy, connect_timeout: Duration) -> Round {
    let server = RawH2::start(move |_| policy()).await;
    let client = client_with(|knobs| {
        knobs.connect_timeout = connect_timeout;
        knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
    });
    let endpoint = server.endpoint();
    let stop = Arc::new(AtomicBool::new(false));
    let bumpers = bumping(&client, &PoolKey::of(&endpoint), &stop);
    let started = Instant::now();
    let mut sends = JoinSet::new();
    for _ in 0..SENDS {
        let (client, endpoint) = (client.clone(), endpoint.clone());
        sends.spawn(async move { client.send(post(&endpoint, "x"), &NoAuthorizer).await });
    }
    let mut failures = Vec::new();
    while let Some(sent) = sends.join_next().await {
        match sent.expect("join") {
            Ok(_) => panic!("a send succeeded against a server that carries nothing"),
            Err(error) => failures.push(error),
        }
    }
    let took = started.elapsed();
    stop.store(true, Ordering::Relaxed);
    for bumper in bumpers {
        bumper.join().expect("bumper");
    }
    Round {
        accepted: server.accepted(),
        took,
        failures,
    }
}

async fn storm(policy: fn() -> Policy, connect_timeout: Duration) -> Vec<Round> {
    let mut rounds = Vec::new();
    for _ in 0..REPETITIONS {
        rounds.push(round(policy, connect_timeout).await);
    }
    rounds
}

fn assert_three_connections_each_time(rounds: &[Round]) {
    for (index, round) in rounds.iter().enumerate() {
        assert_eq!(
            round.accepted, 3,
            "round {index}: a waiter lost the attempt of the connection it waited for ({:?})",
            round.took
        );
        assert_eq!(round.failures.len(), SENDS);
        assert!(
            round
                .failures
                .iter()
                .all(|error| matches!(error, SendError::Connect { .. })),
            "round {index}: {:?}",
            round.failures.first()
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_wakes_of_the_host_while_a_silent_server_serves_a_burst_then_the_sends_use_three_connections_and_three_connect_timeouts()
 {
    let rounds = storm(silent, CONNECT_TIMEOUT).await;
    assert_three_connections_each_time(&rounds);
    for round in &rounds {
        assert!(round.took >= CONNECT_TIMEOUT * 3, "{:?}", round.took);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_wakes_of_the_host_while_connections_end_before_their_settings_then_the_sends_use_three_connections()
 {
    assert_three_connections_each_time(&storm(eof_before_settings, SLOW_CONNECT_TIMEOUT).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_wakes_of_the_host_while_connections_announce_zero_and_end_then_the_sends_use_three_connections()
 {
    assert_three_connections_each_time(&storm(zero_then_eof, SLOW_CONNECT_TIMEOUT).await);
}
