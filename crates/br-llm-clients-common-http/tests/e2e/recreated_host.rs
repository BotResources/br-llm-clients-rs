use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_http::{HttpClient, NoAuthorizer, SendError};

use crate::support::flip_server::{Flip, FlipServer};
use crate::support::pki::Pki;
use crate::support::{client_with, guarded, post};

const HEADERS_TIMEOUT: Duration = Duration::from_secs(8);

fn flip(http2_settings_after: Option<Duration>) -> Flip {
    Flip {
        http1_connections: 1,
        http1_responds_after: Duration::from_millis(300),
        http1_closes_after_responding: Duration::from_millis(100),
        http2_accepts_after: Duration::from_millis(600),
        http2_settings_after,
    }
}

fn client(connect_timeout: Duration) -> HttpClient {
    client_with(|knobs| {
        knobs.connect_timeout = connect_timeout;
        knobs.headers_timeout = HEADERS_TIMEOUT;
        knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
    })
}

struct Timed {
    outcome: Result<u16, SendError>,
}

async fn send_after(
    client: HttpClient,
    server: Arc<FlipServer>,
    start: Duration,
    drop_after: Option<Duration>,
) -> Timed {
    tokio::time::sleep(start).await;
    let sending = async {
        let response = client
            .send(post(&server.endpoint(), "x"), &NoAuthorizer)
            .await?;
        let status = response.status();
        let _ = response.into_parts().2.collect(16).await;
        Ok(status)
    };
    let outcome = match drop_after {
        Some(after) => tokio::time::timeout(after, sending).await.unwrap_or(Ok(0)),
        None => sending.await,
    };
    Timed { outcome }
}

async fn three_sends(
    settings_after: Option<Duration>,
    connect_timeout: Duration,
    second_dropped_after: Option<Duration>,
) -> (Timed, Timed, Timed, usize) {
    let server = Arc::new(FlipServer::start(flip(settings_after)).await);
    let client = client(connect_timeout);
    let spawn = |start: Duration, drop_after: Option<Duration>| {
        tokio::spawn(send_after(
            client.clone(),
            Arc::clone(&server),
            start,
            drop_after,
        ))
    };
    let first = spawn(Duration::ZERO, None);
    let second = spawn(Duration::from_millis(80), second_dropped_after);
    let third = spawn(Duration::from_millis(1080), None);
    let (first, second, third) = guarded(async { tokio::join!(first, second, third) }).await;
    (
        first.expect("join"),
        second.expect("join"),
        third.expect("join"),
        server.accepted(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_dial_started_on_an_http1_host_that_comes_back_as_http2_after_the_last_http1_connection_closed_when_its_settings_come_then_a_send_waiting_for_it_uses_it()
 {
    let (first, second, third, _) = three_sends(
        Some(Duration::from_millis(500)),
        Duration::from_secs(3),
        None,
    )
    .await;
    assert!(matches!(first.outcome, Ok(200)), "{:?}", first.outcome);
    assert!(matches!(second.outcome, Ok(200)), "{:?}", second.outcome);
    assert!(matches!(third.outcome, Ok(200)), "{:?}", third.outcome);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_dial_started_on_an_http1_host_that_comes_back_as_http2_and_never_sends_settings_then_a_send_waiting_for_it_ends_with_connect_when_the_attempts_are_used()
 {
    let (_, _, third, accepted) = three_sends(None, Duration::from_secs(1), None).await;
    assert!(
        matches!(third.outcome, Err(SendError::Connect { .. })),
        "{:?}",
        third.outcome
    );
    assert!(accepted >= 3, "{accepted}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_the_send_that_dialed_is_dropped_and_the_connection_never_sends_settings_then_a_send_waiting_for_it_ends_with_connect_when_the_attempts_are_used()
 {
    let (_, _, third, _) = three_sends(
        None,
        Duration::from_secs(1),
        Some(Duration::from_millis(900)),
    )
    .await;
    assert!(
        matches!(third.outcome, Err(SendError::Connect { .. })),
        "{:?}",
        third.outcome
    );
}
