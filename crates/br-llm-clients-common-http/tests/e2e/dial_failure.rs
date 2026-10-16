use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use br_llm_clients_common_http::NoAuthorizer;
use tokio::net::TcpStream;

use crate::support::burst::{kinds, slowest_of, timed_burst};
use crate::support::frames::response_head;
use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2, TlsFault};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
const ACCEPT_DELAY: Duration = Duration::from_millis(50);
const SLACK: Duration = Duration::from_millis(1000);
const EXTRA_CONNECTIONS: usize = 8;

fn first_connection_holding_a_stream() -> Policy {
    Policy {
        answer: Answer::Frames(|stream| response_head(stream, None, false)),
        ..Policy::announcing(Announce::Limit(1))
    }
}

fn later_connection(fault: TlsFault) -> Policy {
    Policy {
        accept_delay: ACCEPT_DELAY,
        tls_fault: fault,
        ..Policy::announcing(Announce::Limit(1))
    }
}

async fn until_refused(server: &RawH2) {
    let started = std::time::Instant::now();
    while TcpStream::connect(server.addr).await.is_ok() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the listener never closed"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn burst_after_a_failed_capacity_dial(
    fault: TlsFault,
    refused: bool,
    expected: &str,
    sends: usize,
    headers_timeout: Duration,
) {
    let healed = Arc::new(AtomicBool::new(false));
    let server = RawH2::start({
        let healed = Arc::clone(&healed);
        move |index| match index {
            0 => first_connection_holding_a_stream(),
            _ if healed.load(Ordering::SeqCst) => Policy::healthy(),
            _ => later_connection(fault),
        }
    })
    .await;
    let client = trusting(CONNECT_TIMEOUT, headers_timeout);
    let held = client
        .send(post(&server.endpoint(), "held"), &NoAuthorizer)
        .await
        .expect("the first connection holds a stream");
    if refused {
        server.refuse_new_connections();
        until_refused(&server).await;
    }
    let outcomes = timed_burst(&client, &server, sends).await;
    let label = format!("{expected}, {sends} sends, headers {headers_timeout:?}");
    assert_eq!(kinds(&outcomes), vec![expected], "{label}");
    let bound = CONNECT_TIMEOUT + ACCEPT_DELAY + SLACK;
    let slowest = slowest_of(&outcomes);
    assert!(
        slowest.is_some_and(|took| took < bound),
        "{label}: slowest {slowest:?}, bound {bound:?}"
    );
    assert!(
        server.accepted() <= 1 + EXTRA_CONNECTIONS,
        "{label}: {} connections",
        server.accepted()
    );
    if !refused {
        healed.store(true, Ordering::SeqCst);
        let mut response = client
            .send(post(&server.endpoint(), "after"), &NoAuthorizer)
            .await
            .unwrap_or_else(|error| {
                panic!("{label}: a later send is not told the old failure: {error:?}")
            });
        response.body_mut().collect(10).await.expect("body");
    }
    drop(held);
}

async fn every_case(fault: TlsFault, refused: bool, expected: &str) {
    for headers_timeout in [Duration::from_secs(20), Duration::from_secs(2)] {
        for sends in [1, 50, 200] {
            burst_after_a_failed_capacity_dial(fault, refused, expected, sends, headers_timeout)
                .await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_dial_for_one_more_connection_that_ends_the_tls_handshake_then_every_send_that_waited_for_it_fails_with_tls()
 {
    every_case(TlsFault::Close, false, "Tls").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_dial_for_one_more_connection_that_never_answers_then_every_send_that_waited_for_it_fails_with_connect_timeout()
 {
    every_case(TlsFault::Hang, false, "ConnectTimeout").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_dial_for_one_more_connection_that_is_refused_then_every_send_that_waited_for_it_fails_with_connect()
 {
    every_case(TlsFault::None, true, "Connect").await;
}
