use std::time::{Duration, Instant};

use br_llm_clients_common_http::{HttpClient, NoAuthorizer, SendError};

use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};

fn silent() -> Policy {
    Policy {
        answer: Answer::Never,
        ..Policy::announcing(Announce::Nothing)
    }
}

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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_server_that_never_sends_settings_then_fifty_sends_all_end_with_connect_within_three_connect_timeouts()
 {
    let server = RawH2::start(|_| silent()).await;
    let connect_timeout = Duration::from_millis(500);
    let client = trusting(connect_timeout, Duration::from_secs(30));
    let started = Instant::now();
    let outcomes = tokio::time::timeout(Duration::from_secs(20), burst(&client, &server, 50))
        .await
        .expect("the sends hung");
    let elapsed = started.elapsed();
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(outcome, Err(SendError::Connect { .. }))),
        "{outcomes:?}"
    );
    assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
    assert!(
        elapsed < connect_timeout * 3 + Duration::from_secs(3),
        "{elapsed:?}"
    );
    assert!(server.accepted() <= 6, "{} connections", server.accepted());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_connection_whose_settings_never_arrive_then_the_client_closes_it_after_the_connect_timeout()
 {
    let server = RawH2::start(|_| silent()).await;
    let connect_timeout = Duration::from_millis(600);
    let client = trusting(connect_timeout, Duration::from_secs(30));
    let endpoint = server.endpoint();
    let started = Instant::now();
    let sending = tokio::spawn(async move {
        client
            .send(post(&endpoint, "x"), &NoAuthorizer)
            .await
            .map(|_| ())
    });
    while server.closed() == 0 {
        assert!(
            started.elapsed() < connect_timeout + Duration::from_secs(3),
            "the connection was not closed: accepted {}",
            server.accepted()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        started.elapsed() >= connect_timeout,
        "{:?}",
        started.elapsed()
    );
    let outcome = sending.await.expect("join");
    assert!(
        matches!(outcome, Err(SendError::Connect { .. })),
        "{outcome:?}"
    );
    let settled = Instant::now();
    while server.closed() < server.accepted() {
        assert!(
            settled.elapsed() < Duration::from_secs(3),
            "{} of {} connections closed",
            server.closed(),
            server.accepted()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
