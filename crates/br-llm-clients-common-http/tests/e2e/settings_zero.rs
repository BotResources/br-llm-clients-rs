use std::time::Duration;

use crate::support::burst::{kinds, slowest_of, timed_burst};
use crate::support::frames::settings_frame;
use crate::support::gate_kit::trusting;
use crate::support::raw_h2::{Announce, Answer, Ending, Policy, RawH2};

const MAX_CONNECTIONS: usize = 16;

fn frames(limits: &[u32]) -> &'static [u8] {
    let bytes: Vec<u8> = limits
        .iter()
        .flat_map(|limit| settings_frame(Some(*limit)))
        .collect();
    Box::leak(bytes.into_boxed_slice())
}

fn unanswered(announce: Announce) -> Policy {
    Policy {
        answer: Answer::Never,
        ..Policy::announcing(announce)
    }
}

async fn assert_queued_behind_a_later_zero(announce: Announce, name: &str) {
    for sends in [1, 50] {
        let server = RawH2::start(move |_| unanswered(announce)).await;
        let client = trusting(Duration::from_secs(5), Duration::from_millis(600));
        let outcomes = timed_burst(&client, &server, sends).await;
        assert_eq!(
            kinds(&outcomes),
            vec!["HeadersTimeout"],
            "{name}, {sends} sends, {} connections",
            server.accepted()
        );
        let bound = if sends == 1 { 1 } else { MAX_CONNECTIONS };
        assert!(
            server.accepted() <= bound,
            "{name}, {sends} sends: {} connections",
            server.accepted()
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_usable_first_settings_and_a_zero_in_the_same_write_then_the_request_waits_and_costs_no_attempt()
 {
    let announce = Announce::Raw(frames(&[100, 0]));
    assert_queued_behind_a_later_zero(announce, "SETTINGS(100) and SETTINGS(0) in one write").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_usable_first_settings_and_a_zero_a_few_milliseconds_later_then_the_request_waits_and_costs_no_attempt()
 {
    let announce = Announce::Spaced(frames(&[100]), Duration::from_millis(10), frames(&[0]));
    assert_queued_behind_a_later_zero(announce, "SETTINGS(100) then SETTINGS(0)").await;
}

async fn first_zero_then_raised_in_one_write(lost_before: usize) {
    let announce = Announce::Raw(frames(&[0, 100]));
    let server = RawH2::start(move |index| {
        if index < lost_before {
            Policy {
                ends_after: Some((Duration::from_millis(50), Ending::Eof)),
                ..Policy::announcing(Announce::Nothing)
            }
        } else {
            Policy::announcing(announce)
        }
    })
    .await;
    let client = trusting(Duration::from_millis(500), Duration::from_secs(20));
    let outcomes = timed_burst(&client, &server, 20).await;
    assert_eq!(
        kinds(&outcomes),
        vec!["ok"],
        "{lost_before} lost connections, {} connections",
        server.accepted()
    );
    assert!(
        server.accepted() <= lost_before + 3,
        "{}",
        server.accepted()
    );
    assert!(slowest_of(&outcomes).is_some_and(|took| took < Duration::from_secs(10)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_first_zero_raised_in_the_same_write_then_every_send_that_waited_is_served() {
    first_zero_then_raised_in_one_write(0).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_first_zero_raised_in_the_same_write_after_one_lost_attempt_then_every_send_is_served()
 {
    first_zero_then_raised_in_one_write(1).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_first_zero_raised_in_the_same_write_after_two_lost_attempts_then_every_send_is_served()
 {
    first_zero_then_raised_in_one_write(2).await;
}

async fn first_zero_raised_later(raise_after: Duration, in_between: bool) {
    let announce = Announce::Spaced(frames(&[0]), raise_after, frames(&[100]));
    let server = RawH2::start(move |_| Policy::announcing(announce)).await;
    let client = trusting(Duration::from_secs(2), Duration::from_secs(1));
    let early = timed_burst(&client, &server, 20).await;
    assert_eq!(kinds(&early), vec!["Connect"], "before the raise");
    assert!(server.accepted() <= 5, "{} connections", server.accepted());
    if in_between {
        tokio::time::sleep(raise_after / 2).await;
        let middle = timed_burst(&client, &server, 1).await;
        assert_eq!(kinds(&middle), vec!["Connect"], "still before the raise");
        assert!(
            server.accepted() <= MAX_CONNECTIONS,
            "{}",
            server.accepted()
        );
    }
    tokio::time::sleep(raise_after + Duration::from_millis(500)).await;
    let connections = server.accepted();
    let late = timed_burst(&client, &server, 20).await;
    assert_eq!(kinds(&late), vec!["ok"], "after the raise");
    assert_eq!(
        server.accepted(),
        connections,
        "no connection is opened for them"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_first_zero_raised_after_300_ms_then_the_sends_before_it_end_with_connect_and_the_sends_after_it_are_served_on_the_same_connections()
 {
    first_zero_raised_later(Duration::from_millis(300), false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_first_zero_raised_after_3000_ms_then_the_sends_before_it_end_with_connect_without_a_dial_storm_and_the_sends_after_it_are_served()
 {
    first_zero_raised_later(Duration::from_millis(3000), true).await;
}

async fn later_zero_raised(raise_after: Duration, expected: &str) {
    let announce = Announce::Spaced(frames(&[100, 0]), raise_after, frames(&[100]));
    let server = RawH2::start(move |_| Policy::announcing(announce)).await;
    let client = trusting(Duration::from_secs(5), Duration::from_secs(1));
    let outcomes = timed_burst(&client, &server, 20).await;
    assert_eq!(
        kinds(&outcomes),
        vec![expected],
        "{} connections",
        server.accepted()
    );
    assert!(
        server.accepted() <= MAX_CONNECTIONS,
        "{}",
        server.accepted()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_limit_lowered_to_zero_and_raised_after_300_ms_then_the_requests_in_flight_are_served()
 {
    later_zero_raised(Duration::from_millis(300), "ok").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_limit_lowered_to_zero_and_raised_after_3000_ms_then_the_requests_in_flight_end_with_headers_timeout()
 {
    later_zero_raised(Duration::from_millis(3000), "HeadersTimeout").await;
}
