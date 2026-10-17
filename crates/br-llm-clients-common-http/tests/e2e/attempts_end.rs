use std::time::Duration;

use br_llm_clients_common_http::{NoAuthorizer, SendError};

use crate::support::frames::reset_frame;
use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};

const NOT_WRITTEN: &str = "none of it was written";

fn silent() -> Policy {
    Policy {
        answer: Answer::Never,
        ..Policy::announcing(Announce::Nothing)
    }
}

fn refuse(stream: u32) -> Vec<u8> {
    reset_frame(stream, 7)
}

fn refusing_then_full(streams: usize) -> Policy {
    Policy {
        answer: Answer::RefuseAfter(Duration::from_millis(100)),
        lower_to_after: Some((0, streams)),
        ..Policy::healthy()
    }
}

async fn one_send(server: &RawH2) -> SendError {
    let client = trusting(Duration::from_millis(400), Duration::from_secs(30));
    tokio::time::timeout(
        Duration::from_secs(20),
        client.send(post(&server.endpoint(), "x"), &NoAuthorizer),
    )
    .await
    .expect("the send hung")
    .unwrap_err()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_refused_stream_then_a_full_connection_then_two_silent_ones_the_send_is_interrupted()
 {
    for _ in 0..3 {
        let server = RawH2::start(|index| {
            if index == 0 {
                refusing_then_full(1)
            } else {
                silent()
            }
        })
        .await;
        let error = one_send(&server).await;
        assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
        assert!(!error.to_string().contains(NOT_WRITTEN), "{error}");
        assert_eq!(server.streams(), 1);
        assert_eq!(server.accepted(), 3);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_three_silent_connections_the_send_fails_with_connect_because_no_byte_was_written() {
    let server = RawH2::start(|_| silent()).await;
    let error = one_send(&server).await;
    assert!(matches!(error, SendError::Connect { .. }), "{error:?}");
    assert!(error.to_string().contains(NOT_WRITTEN), "{error}");
    assert_eq!(server.streams(), 0);
    assert_eq!(server.accepted(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_stream_refused_three_times_on_one_healthy_connection_the_send_is_interrupted() {
    let server = RawH2::start(|_| Policy {
        answer: Answer::Frames(refuse),
        ..Policy::healthy()
    })
    .await;
    let error = one_send(&server).await;
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
    assert!(!error.to_string().contains(NOT_WRITTEN), "{error}");
    assert_eq!(server.streams(), 3);
    assert_eq!(server.accepted(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_two_sends_refused_together_then_the_one_that_waits_for_the_next_connection_is_interrupted_too()
 {
    for _ in 0..3 {
        let server = RawH2::start(|index| {
            if index == 0 {
                refusing_then_full(2)
            } else {
                silent()
            }
        })
        .await;
        let client = trusting(Duration::from_millis(400), Duration::from_secs(30));
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..2 {
            let (client, endpoint) = (client.clone(), server.endpoint());
            tasks.spawn(async move { client.send(post(&endpoint, "x"), &NoAuthorizer).await });
        }
        let mut outcomes = Vec::new();
        while let Some(outcome) = tokio::time::timeout(Duration::from_secs(20), tasks.join_next())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "the sends hung: {} connections, {} streams, finished {outcomes:?}",
                    server.accepted(),
                    server.streams()
                )
            })
        {
            outcomes.push(outcome.expect("join").unwrap_err());
        }
        assert!(
            outcomes
                .iter()
                .all(|error| matches!(error, SendError::Interrupted { .. })),
            "{outcomes:?}"
        );
        assert_eq!(server.streams(), 2);
    }
}
