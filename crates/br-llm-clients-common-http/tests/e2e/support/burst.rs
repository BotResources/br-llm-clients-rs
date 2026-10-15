use std::time::{Duration, Instant};

use br_llm_clients_common_http::{HttpClient, NoAuthorizer, SendError};

use super::post;
use super::raw_h2::RawH2;

pub type Timed = Vec<(Result<(), SendError>, Duration)>;

pub async fn timed_burst(client: &HttpClient, server: &RawH2, count: usize) -> Timed {
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

pub fn slowest_of(outcomes: &Timed) -> Option<Duration> {
    outcomes.iter().map(|(_, took)| *took).max()
}
