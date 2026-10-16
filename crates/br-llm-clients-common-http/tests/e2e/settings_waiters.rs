use std::time::Duration;

use br_llm_clients_common_http::NoAuthorizer;

use crate::support::burst::{kind_of, kinds, slowest_of, timed_burst};
use crate::support::frames::{goaway_frame, settings_frame};
use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Ending, Policy, RawH2};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
const ACCEPT_DELAY: Duration = Duration::from_millis(50);
const SLACK: Duration = Duration::from_millis(1000);
const MAX_CONNECTIONS: usize = 12;
const PROTOCOL_ERROR: u32 = 1;

fn after_settings(goaway_code: Option<u32>) -> Announce {
    let mut bytes = settings_frame(Some(100));
    if let Some(code) = goaway_code {
        bytes.extend_from_slice(&goaway_frame(0, code, b""));
    }
    Announce::Raw(Box::leak(bytes.into_boxed_slice()))
}

fn shape(announce: Announce, eof_after: Option<Duration>) -> Policy {
    Policy {
        answer: Answer::Never,
        ends_after: eof_after.map(|after| (after, Ending::Eof)),
        accept_delay: ACCEPT_DELAY,
        ..Policy::announcing(announce)
    }
}

async fn assert_one_kind_with_bounded_connections(policy: Policy, name: &str, expected: &str) {
    assert_kinds_with_bounded_connections(policy, name, &[expected], &[1, 50, 200]).await;
}

async fn assert_kinds_with_bounded_connections(
    policy: Policy,
    name: &str,
    allowed: &[&str],
    counts: &[usize],
) {
    for &sends in counts {
        let server = RawH2::start(move |_| policy).await;
        let client = trusting(CONNECT_TIMEOUT, Duration::from_secs(20));
        let outcomes = tokio::time::timeout(
            Duration::from_secs(60),
            timed_burst(&client, &server, sends),
        )
        .await
        .expect("the sends hung");
        let seen = kinds(&outcomes);
        assert!(
            seen.iter().all(|kind| allowed.contains(kind)),
            "{name}, {sends} sends: {seen:?} is not within {allowed:?}, {} connections",
            server.accepted()
        );
        if let [only] = allowed {
            assert_eq!(seen, vec![*only], "{name}, {sends} sends");
        }
        let bound = (CONNECT_TIMEOUT + ACCEPT_DELAY + SLACK) * 3;
        let slowest = slowest_of(&outcomes);
        assert!(
            slowest.is_some_and(|took| took < bound),
            "{name}, {sends} sends: slowest {slowest:?}, bound {bound:?}, {} connections",
            server.accepted()
        );
        assert!(
            server.accepted() <= MAX_CONNECTIONS,
            "{name}, {sends} sends: {} connections",
            server.accepted()
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_limit_of_zero_then_an_eof_then_every_send_ends_with_one_kind_on_few_connections() {
    let policy = shape(Announce::Limit(0), Some(Duration::from_millis(50)));
    assert_one_kind_with_bounded_connections(policy, "MCS=0 then EOF", "Connect").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_goaway_after_settings_then_an_eof_then_every_send_ends_with_one_kind_on_few_connections()
 {
    let policy = shape(after_settings(Some(0)), Some(Duration::from_millis(50)));
    assert_one_kind_with_bounded_connections(policy, "SETTINGS, GOAWAY(0), EOF", "Connect").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_limit_of_zero_on_a_connection_that_stays_open_then_every_send_ends_with_one_kind_on_few_connections()
 {
    let policy = shape(Announce::Limit(0), None);
    assert_one_kind_with_bounded_connections(policy, "MCS=0 open", "Connect").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_goaway_protocol_error_right_after_settings_then_one_send_and_fifty_end_with_the_same_kind()
 {
    let policy = shape(after_settings(Some(PROTOCOL_ERROR)), None);
    assert_one_kind_with_bounded_connections(policy, "GOAWAY PROTOCOL_ERROR", "Connect").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_goaway_protocol_error_right_after_settings_then_an_eof_then_one_send_and_fifty_end_with_the_same_kind()
 {
    let policy = shape(
        after_settings(Some(PROTOCOL_ERROR)),
        Some(Duration::from_millis(50)),
    );
    assert_one_kind_with_bounded_connections(policy, "GOAWAY PROTOCOL_ERROR, EOF", "Connect").await;
}

fn goaway_after_first_headers() -> Policy {
    Policy {
        goaway_on_headers: Some(PROTOCOL_ERROR),
        ..shape(after_settings(None), None)
    }
}

fn goaway_racing_the_first_request() -> Policy {
    Policy {
        goaway_after: Some((Duration::ZERO, PROTOCOL_ERROR)),
        ..shape(after_settings(None), None)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_goaway_after_the_first_request_then_a_send_that_was_taken_is_interrupted() {
    let policy = goaway_after_first_headers();
    assert_kinds_with_bounded_connections(policy, "GOAWAY after HEADERS", &["Interrupted"], &[1])
        .await;
    let allowed = ["Interrupted", "Connect"];
    assert_kinds_with_bounded_connections(policy, "GOAWAY after HEADERS", &allowed, &[50, 200])
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_goaway_racing_the_first_request_then_each_send_is_interrupted_if_taken_and_connect_if_never_taken()
 {
    let allowed = ["Interrupted", "Connect"];
    assert_kinds_with_bounded_connections(
        goaway_racing_the_first_request(),
        "GOAWAY racing the request",
        &allowed,
        &[1, 50, 200],
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_limit_of_zero_kept_open_then_sends_one_after_the_other_never_open_more_than_sixteen_connections_and_then_wait_for_headers_timeout()
 {
    let server = RawH2::start(|_| shape(Announce::Limit(0), None)).await;
    let client = trusting(Duration::from_secs(5), Duration::from_millis(400));
    let mut outcomes = Vec::new();
    for _ in 0..8 {
        let started = std::time::Instant::now();
        let outcome = client
            .send(post(&server.endpoint(), "x"), &NoAuthorizer)
            .await
            .map(|_| ());
        outcomes.push((outcome, started.elapsed()));
    }
    let seen: Vec<_> = outcomes
        .iter()
        .map(|(outcome, _)| kind_of(outcome))
        .collect();
    let expected = [["Connect"; 5].as_slice(), ["HeadersTimeout"; 3].as_slice()].concat();
    assert_eq!(seen, expected);
    assert_eq!(server.accepted(), 16);
}
