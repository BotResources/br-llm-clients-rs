use std::time::{Duration, Instant};

use br_llm_clients_common_http::{Endpoint, HttpClient, NoAuthorizer, SendError};

use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};

async fn burst(
    client: &HttpClient,
    endpoint: &Endpoint,
    count: usize,
) -> Vec<Result<(), SendError>> {
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..count {
        let (client, endpoint) = (client.clone(), endpoint.clone());
        tasks.spawn(async move {
            let mut response = client.send(post(&endpoint, "x"), &NoAuthorizer).await?;
            response
                .body_mut()
                .collect(10)
                .await
                .map(|_| ())
                .map_err(|_| SendError::Protocol {
                    text: "body".to_owned(),
                })
        });
    }
    let mut outcomes = Vec::new();
    while let Some(outcome) = tasks.join_next().await {
        outcomes.push(outcome.expect("join"));
    }
    outcomes
}

fn failures(outcomes: &[Result<(), SendError>]) -> Vec<&SendError> {
    outcomes.iter().filter_map(|o| o.as_ref().err()).collect()
}

fn first_connection_then_healthy(first: Policy) -> impl Fn(usize) -> Policy {
    move |index| if index == 0 { first } else { Policy::healthy() }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_connection_whose_settings_never_arrive_then_the_sends_that_waited_dial_again_after_the_connect_timeout()
 {
    let silent = Policy {
        answer: Answer::Never,
        ..Policy::announcing(Announce::Nothing)
    };
    let server = RawH2::start(first_connection_then_healthy(silent)).await;
    let client = trusting(Duration::from_millis(600), Duration::from_secs(10));
    let started = Instant::now();
    let outcomes = burst(&client, &server.endpoint(), 5).await;
    let elapsed = started.elapsed();
    assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
    assert!(elapsed < Duration::from_secs(4), "{elapsed:?}");
    assert_eq!(server.accepted(), 2);
    let later = Instant::now();
    let single = burst(&client, &server.endpoint(), 1).await;
    assert!(single[0].is_ok(), "{single:?}");
    assert!(later.elapsed() < Duration::from_secs(1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_first_settings_with_a_limit_of_zero_then_the_connection_counts_as_full_and_the_sends_use_another()
 {
    let closed = Policy {
        answer: Answer::Refuse,
        ..Policy::announcing(Announce::Limit(0))
    };
    let server = RawH2::start(first_connection_then_healthy(closed)).await;
    let client = trusting(Duration::from_secs(5), Duration::from_secs(10));
    let started = Instant::now();
    let outcomes = burst(&client, &server.endpoint(), 5).await;
    assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(server.accepted(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_limit_lowered_to_zero_after_traffic_then_the_next_send_opens_another_connection() {
    let lowering = Policy {
        answer: Answer::After(Duration::from_millis(100)),
        lower_to_after: Some((0, 1)),
        ..Policy::healthy()
    };
    let server = RawH2::start(first_connection_then_healthy(lowering)).await;
    let client = trusting(Duration::from_secs(5), Duration::from_secs(10));
    assert!(burst(&client, &server.endpoint(), 1).await[0].is_ok());
    let started = Instant::now();
    let next = burst(&client, &server.endpoint(), 1).await;
    assert!(next[0].is_ok(), "{next:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(server.accepted(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_requests_queued_behind_a_lowered_limit_then_every_send_ends_within_the_headers_timeout()
 {
    let silent_and_lowering = Policy {
        answer: Answer::Never,
        lower_to_after: Some((0, 1)),
        ..Policy::healthy()
    };
    let server = RawH2::start(move |_| silent_and_lowering).await;
    let headers_timeout = Duration::from_millis(800);
    let client = trusting(Duration::from_secs(5), headers_timeout);
    let started = Instant::now();
    let outcomes = burst(&client, &server.endpoint(), 12).await;
    assert!(
        started.elapsed() < headers_timeout * 3,
        "{:?}",
        started.elapsed()
    );
    assert!(
        failures(&outcomes)
            .iter()
            .all(|e| matches!(e, SendError::HeadersTimeout { .. })),
        "{outcomes:?}"
    );
    assert_eq!(failures(&outcomes).len(), 12);
    assert!(server.streams() >= 1);
}

#[tokio::test]
async fn given_a_server_that_refuses_each_attempt_late_then_every_attempt_has_its_own_headers_deadline()
 {
    let headers_timeout = Duration::from_millis(800);
    let refusing = Policy {
        answer: Answer::RefuseAfter(Duration::from_millis(600)),
        ..Policy::healthy()
    };
    let server = RawH2::start(move |_| refusing).await;
    let client = trusting(Duration::from_secs(5), headers_timeout);
    let started = Instant::now();
    let outcome = burst(&client, &server.endpoint(), 1).await;
    let elapsed = started.elapsed();
    assert!(
        matches!(outcome[0], Err(SendError::Interrupted { .. })),
        "{outcome:?}"
    );
    assert_eq!(server.streams(), 3);
    assert!(elapsed > headers_timeout, "{elapsed:?}");
    assert!(elapsed < headers_timeout * 3, "{elapsed:?}");
}
