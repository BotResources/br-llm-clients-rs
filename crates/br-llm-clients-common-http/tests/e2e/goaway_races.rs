use std::time::Instant;

use crate::support::pki::Pki;
use crate::support::probe::{Probe, Script};
use crate::support::spy::Spy;
use crate::support::trusting_client;
use br_llm_clients_common_http::Request;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_sends_racing_a_goaway_then_none_is_left_waiting_for_the_headers_timeout() {
    for round in 0..100 {
        let probe = Probe::start(Script::GoAway {
            first_connections: 6,
            last_stream: 0,
            code: 0,
        })
        .await;
        let client = trusting_client(&Pki::trusted().ca_pem);
        let started = Instant::now();
        let mut tasks = tokio::task::JoinSet::new();
        for i in 0..120 {
            let client = client.clone();
            let request = Request::new(probe.endpoint(), "payload")
                .with_header("x-id", &format!("r{i}"))
                .expect("header");
            tasks.spawn(async move {
                match client.send(request, &Spy::returning(&[])).await {
                    Ok(mut response) => response
                        .body_mut()
                        .collect(10)
                        .await
                        .map(|_| ())
                        .map_err(|_| "body".to_owned()),
                    Err(error) => Err(format!("{error:?}")),
                }
            });
        }
        let mut odd = Vec::new();
        while let Some(outcome) = tasks.join_next().await {
            let expected = |kind: &str| {
                kind.starts_with("Interrupted")
                    || kind.starts_with("Connect { text: \"no connection could carry")
                    || kind == "body"
            };
            if let Err(kind) = outcome.expect("join")
                && !expected(&kind)
            {
                odd.push(kind);
            }
        }
        assert!(
            odd.is_empty(),
            "round {round}: {} odd, e.g. {:?}",
            odd.len(),
            &odd[..odd.len().min(2)]
        );
        assert!(
            started.elapsed().as_secs() < 8,
            "round {round}: {:?}",
            started.elapsed()
        );
    }
}
