use std::time::{Duration, Instant};

use br_llm_clients_common_http::{NoAuthorizer, SendError};

use crate::support::frames::goaway_frame;
use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Ending, Policy, RawH2};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(600);
const ATTEMPT_BOUND: Duration = Duration::from_millis(1500);
const MAX_CONNECTIONS: usize = 30;

fn ending(announce: Announce, ends: Option<(Duration, Ending)>) -> Policy {
    Policy {
        answer: Answer::Never,
        ends_after: ends,
        ..Policy::announcing(announce)
    }
}

fn goaway_only() -> Announce {
    let bytes: &'static [u8] = Box::leak(goaway_frame(0, 0, b"").into_boxed_slice());
    Announce::Raw(bytes)
}

async fn timed_burst(server: &RawH2, count: usize) -> Vec<(Result<(), SendError>, Duration)> {
    let client = trusting(CONNECT_TIMEOUT, Duration::from_secs(60));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..count {
        let (client, endpoint) = (client.clone(), server.endpoint());
        tasks.spawn(async move {
            let started = Instant::now();
            let outcome = client
                .send(post(&endpoint, "x"), &NoAuthorizer)
                .await
                .map(|_| ());
            (outcome, started.elapsed())
        });
    }
    let mut outcomes = Vec::new();
    while let Some(outcome) = tasks.join_next().await {
        outcomes.push(outcome.expect("join"));
    }
    outcomes
}

async fn assert_every_send_ends_with_connect(policy: Policy, sends: usize, shape: &str) {
    let server = RawH2::start(move |_| policy).await;
    let outcomes = tokio::time::timeout(Duration::from_secs(30), timed_burst(&server, sends))
        .await
        .expect("the sends hung");
    let slowest = outcomes.iter().map(|(_, took)| *took).max();
    assert!(
        outcomes
            .iter()
            .all(|(outcome, _)| matches!(outcome, Err(SendError::Connect { .. }))),
        "{shape}, {sends} sends: {outcomes:?}"
    );
    let bound = ATTEMPT_BOUND * 3;
    assert!(
        slowest.is_some_and(|took| took < bound),
        "{shape}, {sends} sends: slowest {slowest:?}, bound {bound:?}"
    );
    assert!(
        server.accepted() <= MAX_CONNECTIONS,
        "{shape}, {sends} sends: {} connections",
        server.accepted()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_connection_that_ends_before_its_first_settings_then_every_send_ends_with_connect_after_three_attempts()
 {
    for after in [50u64, 250] {
        for sends in [50usize, 200] {
            let ends = Some((Duration::from_millis(after), Ending::Eof));
            let policy = ending(Announce::Nothing, ends);
            let shape = format!("EOF after {after} ms");
            assert_every_send_ends_with_connect(policy, sends, &shape).await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_connection_reset_before_its_first_settings_then_every_send_ends_with_connect_after_three_attempts()
 {
    for sends in [50usize, 200] {
        let ends = Some((Duration::from_millis(50), Ending::Reset));
        let policy = ending(Announce::Nothing, ends);
        assert_every_send_ends_with_connect(policy, sends, "reset after 50 ms").await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_goaway_as_the_only_frame_then_every_send_ends_with_connect_after_three_attempts() {
    for sends in [50usize, 200] {
        assert_every_send_ends_with_connect(
            ending(goaway_only(), None),
            sends,
            "GOAWAY and nothing else",
        )
        .await;
        let ends = Some((Duration::from_millis(50), Ending::Eof));
        assert_every_send_ends_with_connect(ending(goaway_only(), ends), sends, "GOAWAY then EOF")
            .await;
    }
}
