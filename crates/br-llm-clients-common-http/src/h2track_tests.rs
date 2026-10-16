use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use hyper::client::conn::http2;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::io::AsyncWriteExt;

use super::{Gone, track_streams};
use crate::h2state::H2State;
use crate::h2state_tests::{bare_state, pooled_state, signalled_state, state_with_limit};
use crate::payload::Payload;
use crate::settled::Settled;
use crate::support::frames::{SETTINGS, frame};

struct ConnectionProbe {
    state: Arc<H2State>,
    retired_when_dropped: Arc<AtomicBool>,
}

impl Future for ConnectionProbe {
    type Output = Result<(), hyper::Error>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

impl Drop for ConnectionProbe {
    fn drop(&mut self) {
        self.retired_when_dropped
            .store(self.state.is_retired(), Ordering::SeqCst);
    }
}

fn probe(state: &Arc<H2State>) -> (ConnectionProbe, Arc<AtomicBool>) {
    let seen = Arc::new(AtomicBool::new(false));
    let probe = ConnectionProbe {
        state: Arc::clone(state),
        retired_when_dropped: Arc::clone(&seen),
    };
    (probe, seen)
}

#[tokio::test(start_paused = true)]
async fn given_a_connection_closed_because_its_first_settings_did_not_come_then_it_is_retired_before_the_connection_is_dropped()
 {
    let state = bare_state();
    let (connection, retired_when_dropped) = probe(&state);
    track_streams(connection, Arc::clone(&state), Duration::from_secs(7)).await;
    assert!(retired_when_dropped.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn given_a_tracked_connection_dropped_in_the_middle_then_it_is_retired_before_the_connection_is_dropped()
 {
    let state = state_with_limit(10);
    let (connection, retired_when_dropped) = probe(&state);
    let tracked = track_streams(connection, Arc::clone(&state), Duration::from_secs(7));
    assert!(
        tokio::time::timeout(Duration::from_secs(60), tracked)
            .await
            .is_err()
    );
    assert!(retired_when_dropped.load(Ordering::SeqCst));
}

#[test]
fn given_a_connection_that_ends_then_it_is_retired_before_anyone_is_told_it_is_gone() {
    let state = state_with_limit(10);
    let permit = state.force_acquire();
    let receiver = permit.gone();
    let holding = receiver.borrow();
    let ending = Gone(std::sync::Arc::clone(&state));
    let dropper = std::thread::spawn(move || drop(ending));
    let started = Instant::now();
    while !state.is_retired() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the gone signal was sent before the connection was retired"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!*holding);
    drop(holding);
    dropper.join().expect("dropper");
    assert!(*receiver.borrow());
}

#[tokio::test]
async fn given_a_connection_that_ends_before_its_first_settings_then_a_send_that_waited_for_it_has_lost_an_attempt()
 {
    let state = bare_state();
    track_streams(
        async { Ok(()) },
        std::sync::Arc::clone(&state),
        Duration::from_secs(60),
    )
    .await;
    assert!(state.limit_unknown());
    assert!(state.violation().is_none());
    assert!(state.is_retired());
    assert!(matches!(state.settled(), Some(Settled::Spent)));
}

#[tokio::test]
async fn given_a_connection_that_ends_after_its_first_settings_then_it_is_retired_with_its_limit_known()
 {
    let state = state_with_limit(10);
    track_streams(
        async { Ok(()) },
        std::sync::Arc::clone(&state),
        Duration::from_secs(60),
    )
    .await;
    assert!(!state.limit_unknown());
    assert!(state.is_retired());
}

#[tokio::test(start_paused = true)]
async fn given_a_connection_whose_first_settings_do_not_come_within_the_wait_then_it_is_closed_and_spent()
 {
    let state = bare_state();
    let started = tokio::time::Instant::now();
    let tracked = track_streams(
        std::future::pending::<Result<(), hyper::Error>>(),
        std::sync::Arc::clone(&state),
        Duration::from_secs(7),
    );
    tokio::time::timeout(Duration::from_secs(60), tracked)
        .await
        .expect("the wait for the first settings never ends");
    assert_eq!(started.elapsed(), Duration::from_secs(7));
    assert!(state.is_retired());
    assert!(matches!(state.settled(), Some(Settled::Spent)));
}

#[tokio::test(start_paused = true)]
async fn given_a_connection_whose_first_settings_came_then_it_is_not_closed_when_the_wait_passes() {
    let state = state_with_limit(10);
    let tracked = track_streams(
        std::future::pending::<Result<(), hyper::Error>>(),
        std::sync::Arc::clone(&state),
        Duration::from_secs(7),
    );
    tokio::pin!(tracked);
    tokio::select! {
        () = &mut tracked => panic!("the connection was closed"),
        () = tokio::time::sleep(Duration::from_secs(60)) => {}
    }
    assert!(!state.is_retired());
}

#[test]
fn given_a_connection_that_ends_then_the_sends_waiting_on_the_host_are_woken() {
    let (state, signal) = signalled_state();
    let changes = signal.subscribe();
    assert!(!changes.has_changed().expect("open"));
    drop(Gone(Arc::clone(&state)));
    assert!(changes.has_changed().expect("open"));
}

#[test]
fn given_a_connection_that_ends_then_the_sends_holding_a_stream_of_it_are_told_it_is_gone() {
    let state = state_with_limit(10);
    let permit = state.force_acquire();
    let gone = permit.gone();
    assert!(!*gone.borrow());
    drop(Gone(Arc::clone(&state)));
    assert!(*gone.borrow());
}

#[tokio::test]
async fn given_a_connection_that_ended_by_itself_then_its_sender_stays_open_until_the_connection_is_dropped()
 {
    let (client_io, server_io) = tokio::io::duplex(65536);
    let (sender, connection) = http2::Builder::new(TokioExecutor::new())
        .handshake::<_, Payload>(TokioIo::new(client_io))
        .await
        .expect("handshake");
    let mut connection = std::pin::pin!(connection);
    drop(server_io);
    assert!(connection.as_mut().await.is_err());
    assert!(!sender.is_closed());
}

#[tokio::test]
async fn given_a_connection_dropped_then_its_sender_is_closed() {
    let (client_io, _server_io) = tokio::io::duplex(65536);
    let (sender, connection) = http2::Builder::new(TokioExecutor::new())
        .handshake::<_, Payload>(TokioIo::new(client_io))
        .await
        .expect("handshake");
    assert!(!sender.is_closed());
    drop(connection);
    assert!(sender.is_closed());
}

#[tokio::test]
async fn given_a_connection_that_breaks_a_rule_then_the_violation_is_recorded_before_the_connection_is_retired()
 {
    let (pool, state) = pooled_state();
    let seen = Arc::new(Mutex::new(Vec::new()));
    pool.hooks.on_end_step({
        let (state, seen) = (Arc::clone(&state), Arc::clone(&seen));
        move || seen.lock().expect("seen").push(state.violation().is_some())
    });
    let (client_io, mut server_io) = tokio::io::duplex(65536);
    let (_sender, connection) = http2::Builder::new(TokioExecutor::new())
        .handshake::<_, Payload>(TokioIo::new(client_io))
        .await
        .expect("handshake");
    server_io
        .write_all(&frame(SETTINGS, 0, 1, &[]))
        .await
        .expect("write");
    track_streams(connection, Arc::clone(&state), Duration::from_secs(60)).await;
    assert!(state.violation().is_some());
    assert_eq!(*seen.lock().expect("seen"), vec![true]);
}
