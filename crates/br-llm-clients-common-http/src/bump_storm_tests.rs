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
const ROUND_LIMIT: Duration = Duration::from_secs(30);
const STORM_LIMIT: Duration = Duration::from_secs(240);

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

struct Bumpers {
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl Drop for Bumpers {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

fn bumping(client: &HttpClient, key: &PoolKey) -> Bumpers {
    let stop = Arc::new(AtomicBool::new(false));
    let threads = (0..BUMPERS)
        .map(|_| {
            let (client, key, stop) = (client.clone(), key.clone(), Arc::clone(&stop));
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
        .collect();
    Bumpers { stop, threads }
}

async fn round(policy: fn() -> Policy, connect_timeout: Duration) -> Round {
    let server = RawH2::start(move |_| policy()).await;
    let client = client_with(|knobs| {
        knobs.connect_timeout = connect_timeout;
        knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
    });
    let endpoint = server.endpoint();
    let bumpers = bumping(&client, &PoolKey::of(&endpoint));
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
    drop(bumpers);
    Round {
        accepted: server.accepted(),
        took,
        failures,
    }
}

async fn storm(
    shape: &str,
    policy: fn() -> Policy,
    connect_timeout: Duration,
    check: impl Fn(usize, &Round),
) {
    let rounds = async {
        for index in 0..REPETITIONS {
            let ended = tokio::time::timeout(ROUND_LIMIT, round(policy, connect_timeout)).await;
            let ended = ended.unwrap_or_else(|_| {
                panic!("{shape}: round {index} did not end within {ROUND_LIMIT:?}")
            });
            check(index, &ended);
        }
    };
    tokio::time::timeout(STORM_LIMIT, rounds)
        .await
        .unwrap_or_else(|_| {
            panic!("{shape}: {REPETITIONS} rounds did not end within {STORM_LIMIT:?}")
        });
}

fn assert_three_connections(index: usize, round: &Round) {
    assert_eq!(
        round.accepted, 3,
        "round {index}: a waiter lost the attempt of the connection it waited for ({:?})",
        round.took
    );
    assert_eq!(round.failures.len(), SENDS, "round {index}");
    assert!(
        round
            .failures
            .iter()
            .all(|error| matches!(error, SendError::Connect { .. })),
        "round {index}: {:?}",
        round.failures.first()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_wakes_of_the_host_while_a_silent_server_serves_a_burst_then_the_sends_use_three_connections_and_three_connect_timeouts()
 {
    storm(
        "a silent server",
        silent,
        CONNECT_TIMEOUT,
        |index, round| {
            assert_three_connections(index, round);
            assert!(
                round.took >= CONNECT_TIMEOUT * 3,
                "round {index}: {:?}",
                round.took
            );
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_wakes_of_the_host_while_connections_end_before_their_settings_then_the_sends_use_three_connections()
 {
    storm(
        "connections that end before their settings",
        eof_before_settings,
        SLOW_CONNECT_TIMEOUT,
        assert_three_connections,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_wakes_of_the_host_while_connections_announce_zero_and_end_then_the_sends_use_three_connections()
 {
    storm(
        "connections that announce zero and end",
        zero_then_eof,
        SLOW_CONNECT_TIMEOUT,
        assert_three_connections,
    )
    .await;
}
