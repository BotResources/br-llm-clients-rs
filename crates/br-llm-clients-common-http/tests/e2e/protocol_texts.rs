use std::time::Duration;

use br_llm_clients_common_http::{NoAuthorizer, SendError};

use crate::support::frames::{DATA, frame};
use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};

fn data_on_stream_zero() -> &'static [u8] {
    Box::leak(frame(DATA, 0, 0, b"x").into_boxed_slice())
}

fn broken_rule_after_the_request(stream: u32) -> Vec<u8> {
    let _ = stream;
    frame(DATA, 0, 0, b"x")
}

async fn texts(server: &RawH2, sends: usize) -> Vec<String> {
    let client = trusting(Duration::from_secs(5), Duration::from_secs(10));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..sends {
        let (client, endpoint) = (client.clone(), server.endpoint());
        tasks.spawn(async move { client.send(post(&endpoint, "x"), &NoAuthorizer).await });
    }
    let mut texts = Vec::new();
    while let Some(outcome) = tasks.join_next().await {
        match outcome.expect("join").unwrap_err() {
            SendError::Protocol { text } => texts.push(text),
            other => panic!("{other:?}"),
        }
    }
    texts
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_the_same_http2_fault_met_by_a_waiter_and_by_a_request_then_the_protocol_text_is_the_same()
 {
    let bytes = data_on_stream_zero();
    let waiting = RawH2::start(move |_| Policy {
        answer: Answer::Never,
        ..Policy::announcing(Announce::RawAfter(Duration::from_millis(300), bytes))
    })
    .await;
    let carried = RawH2::start(|_| Policy {
        answer: Answer::Frames(broken_rule_after_the_request),
        ..Policy::healthy()
    })
    .await;
    let waited = texts(&waiting, 8).await;
    let request = texts(&carried, 1).await;
    let suffix = format!(" (https://127.0.0.1:{}/x)", waiting.addr.port());
    assert!(waited[0].ends_with(&suffix), "{}", waited[0]);
    assert!(waited.iter().all(|text| *text == waited[0]), "{waited:?}");
    let expected = waited[0].replace(
        &waiting.addr.port().to_string(),
        &carried.addr.port().to_string(),
    );
    assert_eq!(request[0], expected);
}
