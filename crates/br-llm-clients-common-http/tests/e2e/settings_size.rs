use std::time::Duration;

use br_llm_clients_common_http::{HttpClient, NoAuthorizer, SendError};

use crate::support::frames::{SETTINGS, frame};
use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};

async fn burst(client: &HttpClient, server: &RawH2, count: usize) -> Vec<Result<(), SendError>> {
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..count {
        let (client, endpoint) = (client.clone(), server.endpoint());
        tasks.spawn(async move {
            client
                .send(post(&endpoint, "x"), &NoAuthorizer)
                .await
                .map(|_| ())
        });
    }
    let mut outcomes = Vec::new();
    while let Some(outcome) = tasks.join_next().await {
        outcomes.push(outcome.expect("join"));
    }
    outcomes
}

fn answering(announce: Announce) -> Policy {
    Policy {
        answer: Answer::After(Duration::from_millis(250)),
        ..Policy::announcing(announce)
    }
}

async fn peak_with(announce: Announce) -> (usize, usize) {
    let server = RawH2::start(move |_| answering(announce)).await;
    let client = trusting(Duration::from_secs(5), Duration::from_secs(10));
    let outcomes = tokio::time::timeout(Duration::from_secs(20), burst(&client, &server, 6))
        .await
        .expect("the sends hung");
    assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
    (server.peak_streams_on_one_connection(), server.accepted())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_settings_frame_of_16380_bytes_ending_with_a_limit_of_one_then_a_connection_never_carries_two_streams()
 {
    let (peak, accepted) = peak_with(Announce::Padded {
        filler: 2729,
        limit: 1,
    })
    .await;
    assert_eq!(peak, 1);
    assert!(accepted >= 2, "{accepted} connections");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_settings_frame_of_1032_bytes_ending_with_a_limit_of_one_then_a_connection_never_carries_two_streams()
 {
    let (peak, accepted) = peak_with(Announce::Padded {
        filler: 171,
        limit: 1,
    })
    .await;
    assert_eq!(peak, 1);
    assert!(accepted >= 2, "{accepted} connections");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_large_first_settings_frame_without_a_limit_then_the_connection_is_shared() {
    let filler: Vec<u8> = (0..2000).flat_map(|_| [0x77, 0x77, 0, 0, 0, 9]).collect();
    let bytes: &'static [u8] = Box::leak(frame(SETTINGS, 0, 0, &filler).into_boxed_slice());
    let (peak, accepted) = peak_with(Announce::Raw(bytes)).await;
    assert!(peak >= 2, "{peak}");
    assert_eq!(accepted, 1);
}
