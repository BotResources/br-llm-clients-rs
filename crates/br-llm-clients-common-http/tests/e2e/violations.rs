use std::time::{Duration, Instant};

use br_llm_clients_common_http::{HttpClient, NoAuthorizer, SendError};

use crate::support::frames::{ACK, DATA, HEADERS, SETTINGS, frame, settings_frame};
use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Protocol,
    Connect,
}

fn kind_of(error: &SendError) -> Option<Kind> {
    match error {
        SendError::Protocol { .. } => Some(Kind::Protocol),
        SendError::Connect { .. } => Some(Kind::Connect),
        _ => None,
    }
}

fn leaked(bytes: Vec<u8>) -> &'static [u8] {
    Box::leak(bytes.into_boxed_slice())
}

fn entries(pairs: &[(u16, u32)]) -> Vec<u8> {
    pairs
        .iter()
        .flat_map(|(id, value)| {
            let mut entry = id.to_be_bytes().to_vec();
            entry.extend_from_slice(&value.to_be_bytes());
            entry
        })
        .collect()
}

fn after_valid_settings(more: Vec<u8>) -> Vec<u8> {
    let mut bytes = settings_frame(Some(100));
    bytes.extend_from_slice(&more);
    bytes
}

struct Fault {
    name: &'static str,
    bytes: Vec<u8>,
    expected: Kind,
}

fn faults() -> Vec<Fault> {
    let broken = |name, bytes, expected| Fault {
        name,
        bytes,
        expected,
    };
    vec![
        broken(
            "ENABLE_PUSH=2",
            frame(SETTINGS, 0, 0, &entries(&[(2, 2)])),
            Kind::Protocol,
        ),
        broken(
            "INITIAL_WINDOW_SIZE=2^31",
            frame(SETTINGS, 0, 0, &entries(&[(4, 1 << 31)])),
            Kind::Protocol,
        ),
        broken(
            "MAX_FRAME_SIZE=1",
            frame(SETTINGS, 0, 0, &entries(&[(5, 1)])),
            Kind::Protocol,
        ),
        broken(
            "MAX_FRAME_SIZE=2^24",
            frame(SETTINGS, 0, 0, &entries(&[(5, 1 << 24)])),
            Kind::Protocol,
        ),
        broken(
            "SETTINGS length not a multiple of 6",
            frame(SETTINGS, 0, 0, &[0, 3, 0, 0, 0, 5, 0]),
            Kind::Protocol,
        ),
        broken(
            "SETTINGS on stream 3",
            frame(SETTINGS, 0, 3, &entries(&[(3, 5)])),
            Kind::Protocol,
        ),
        broken(
            "SETTINGS ACK with a payload",
            frame(SETTINGS, ACK, 0, &entries(&[(3, 5)])),
            Kind::Protocol,
        ),
        broken(
            "frame larger than the maximum frame size",
            after_valid_settings(frame(DATA, 0, 1, &vec![0; 16385])),
            Kind::Protocol,
        ),
        broken(
            "DATA on stream 0",
            after_valid_settings(frame(DATA, 0, 0, b"x")),
            Kind::Protocol,
        ),
        broken(
            "HEADERS that cannot be decoded",
            after_valid_settings(frame(HEADERS, 4, 2, &[0xff; 20])),
            Kind::Protocol,
        ),
        broken(
            "first SETTINGS gives a limit of 5 then 0 in one frame",
            frame(SETTINGS, 0, 0, &entries(&[(3, 5), (3, 0)])),
            Kind::Connect,
        ),
        broken(
            "SETTINGS ACK before any SETTINGS",
            frame(SETTINGS, ACK, 0, &[]),
            Kind::Protocol,
        ),
        broken("no frame at all", Vec::new(), Kind::Connect),
    ]
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
async fn given_a_server_that_breaks_an_http2_rule_then_every_send_fails_with_the_same_kind_in_bounded_time()
 {
    for fault in faults() {
        for sends in [1usize, 6] {
            let bytes = leaked(fault.bytes.clone());
            let server = RawH2::start(move |_| Policy {
                answer: Answer::Never,
                ..Policy::announcing(Announce::Raw(bytes))
            })
            .await;
            let client = trusting(Duration::from_millis(500), Duration::from_secs(10));
            let started = Instant::now();
            let outcomes =
                tokio::time::timeout(Duration::from_secs(20), burst(&client, &server, sends))
                    .await
                    .expect("the sends hung");
            let elapsed = started.elapsed();
            let kinds: Vec<_> = outcomes
                .iter()
                .map(|outcome| outcome.as_ref().err().and_then(kind_of))
                .collect();
            assert!(
                kinds.iter().all(|kind| *kind == Some(fault.expected)),
                "{} with {sends} sends: expected {:?}, got {outcomes:?}",
                fault.name,
                fault.expected
            );
            assert!(
                elapsed < Duration::from_secs(6),
                "{} with {sends} sends took {elapsed:?}",
                fault.name
            );
            assert!(
                server.accepted() <= sends.saturating_mul(3),
                "{} with {sends} sends opened {} connections",
                fault.name,
                server.accepted()
            );
        }
    }
}

#[tokio::test]
async fn given_a_refused_tcp_connection_then_the_send_fails_with_connect_not_protocol() {
    let client = trusting(Duration::from_secs(2), Duration::from_secs(5));
    let endpoint = crate::support::closed_port_endpoint(true);
    let error = client
        .send(post(&endpoint, "x"), &NoAuthorizer)
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Connect { .. }), "{error:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_rule_broken_while_sends_wait_for_the_first_settings_then_they_all_fail_with_protocol_on_that_one_connection()
 {
    let bytes = leaked(frame(DATA, 0, 0, b"x"));
    let server = RawH2::start(move |_| Policy {
        answer: Answer::Never,
        ..Policy::announcing(Announce::RawAfter(Duration::from_millis(300), bytes))
    })
    .await;
    let client = trusting(Duration::from_secs(5), Duration::from_secs(10));
    let outcomes = tokio::time::timeout(Duration::from_secs(20), burst(&client, &server, 20))
        .await
        .expect("the sends hung");
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(outcome, Err(SendError::Protocol { .. }))),
        "{outcomes:?}"
    );
    assert_eq!(server.accepted(), 1);
}
