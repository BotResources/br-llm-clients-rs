use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::acquire::{Acquired, Acquisition, acquire};
use crate::acquire_tests::{CONNECT_TIMEOUT, Outcome, Rig, outcome};
use crate::dialer::DialFailure;
use crate::endpoint::Endpoint;
use crate::pool::{Plan, PoolKey};
use crate::redact::Redactor;
use crate::send_error::SendError;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};

async fn still_waiting_after(waiter: &mut JoinHandle<Outcome>, span: Duration) {
    assert!(
        tokio::time::timeout(span, &mut *waiter).await.is_err(),
        "the waiter ended while the dial it waits for was still open"
    );
}

async fn failure_of(waiter: JoinHandle<Outcome>) -> SendError {
    let Outcome::Failed(error) = outcome(waiter).await else {
        panic!("the waiter fails");
    };
    error
}

fn bump_host(rig: &Rig) {
    let signal = rig
        .shared
        .pool
        .lock()
        .get(&rig.key)
        .map(|host| Arc::clone(&host.signal));
    if let Some(signal) = signal {
        crate::transport::bump(&signal);
    }
}

#[tokio::test(start_paused = true)]
async fn given_a_send_waiting_behind_a_dial_that_is_still_open_then_it_keeps_no_timer_of_its_own_and_gets_the_failure_of_that_dial()
 {
    let rig = Rig::offline();
    let dialer = rig.dialer();
    let mut waiter = rig.waiter();
    rig.waiting(1).await;
    still_waiting_after(&mut waiter, CONNECT_TIMEOUT * 10).await;
    dialer.fail(DialFailure::Connect("refused".to_owned()));
    let error = failure_of(waiter).await;
    assert!(matches!(error, SendError::Connect { .. }), "{error:?}");
}

#[tokio::test(start_paused = true)]
async fn given_a_send_waiting_behind_a_dial_that_then_times_out_then_it_gets_that_connect_timeout()
{
    let rig = Rig::offline();
    let dialer = rig.dialer();
    let mut waiter = rig.waiter();
    rig.waiting(1).await;
    still_waiting_after(&mut waiter, CONNECT_TIMEOUT * 3).await;
    dialer.fail(DialFailure::Timeout);
    let error = failure_of(waiter).await;
    assert!(
        matches!(error, SendError::ConnectTimeout { .. }),
        "{error:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn given_a_send_waiting_behind_one_dial_and_then_behind_another_then_it_keeps_no_timer_and_gets_the_failure_of_the_second()
 {
    let rig = Rig::offline();
    let first = rig.dialer();
    let mut waiter = rig.waiter();
    rig.waiting(1).await;
    still_waiting_after(&mut waiter, CONNECT_TIMEOUT * 4).await;
    drop(first);
    let second = rig.dialer();
    still_waiting_after(&mut waiter, CONNECT_TIMEOUT * 4).await;
    second.fail(DialFailure::Tls("refused".to_owned()));
    let error = failure_of(waiter).await;
    assert!(matches!(error, SendError::Tls { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_send_that_waited_behind_a_dial_and_then_dials_itself_then_its_own_connect_timeout_starts_with_its_own_dial()
 {
    let connect_timeout = Duration::from_secs(2);
    let rig = Rig::slow_silent_server(Duration::from_millis(1500), connect_timeout).await;
    let first = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    drop(first);
    assert!(matches!(outcome(waiter).await, Outcome::Ready));
}

#[tokio::test(start_paused = true)]
async fn given_wakes_that_change_nothing_while_a_send_waits_behind_a_dial_then_it_keeps_waiting() {
    let rig = Rig::offline();
    let dialer = rig.dialer();
    let mut waiter = rig.waiter();
    rig.waiting(1).await;
    for _ in 0..8 {
        still_waiting_after(&mut waiter, CONNECT_TIMEOUT).await;
        bump_host(&rig);
    }
    dialer.fail(DialFailure::Connect("refused".to_owned()));
    let error = failure_of(waiter).await;
    assert!(matches!(error, SendError::Connect { .. }), "{error:?}");
}

#[tokio::test(start_paused = true)]
async fn given_a_connect_timeout_text_when_the_path_holds_a_secret_then_the_secret_is_removed() {
    let endpoint = Endpoint::https("https://h.example/s3cr3t").expect("endpoint");
    let rig = Rig::timed(endpoint.clone(), None, CONNECT_TIMEOUT, CONNECT_TIMEOUT);
    let dialer = rig.dialer();
    let redactor = Redactor::new(["s3cr3t"]);
    let key = PoolKey::of(&endpoint);
    let (outcome, ()) = tokio::join!(acquire(&rig.shared, &endpoint, &key, &redactor), async {
        rig.waiting(1).await;
        dialer.fail(DialFailure::Timeout);
    });
    let text = match outcome {
        Err(SendError::ConnectTimeout { text }) => text,
        Err(other) => panic!("{other:?}"),
        Ok(Acquisition::Spent) => panic!("spent"),
        Ok(Acquisition::Ready(_)) => panic!("ready"),
    };
    assert!(text.contains("h.example"), "{text}");
    assert!(!text.contains("s3cr3t"), "{text}");
}

#[tokio::test]
async fn given_a_send_that_waited_for_a_stream_and_then_waits_behind_another_dial_then_its_headers_deadline_restarts()
 {
    let headers_timeout = Duration::from_secs(2);
    let server = RawH2::start(|_| Policy {
        answer: Answer::Never,
        ..Policy::announcing(Announce::Nothing)
    })
    .await;
    let rig = Rig::timed(
        server.endpoint(),
        Some(server),
        CONNECT_TIMEOUT,
        headers_timeout,
    );
    let mut kept = Vec::new();
    for _ in 0..16 {
        let (state, conn) = rig.land(rig.dialer()).await;
        state.apply_settings(Some(1));
        kept.push((state, conn));
    }
    let waiter = tokio::spawn(rig.acquisition());
    tokio::time::sleep(headers_timeout * 6 / 10).await;
    for (state, _) in &kept {
        state.retire();
        state.mark_gone();
    }
    let Plan::Dial(dialer) = rig.shared.pool.plan(&rig.key, None) else {
        panic!("the test takes the dial gate");
    };
    kept.first().expect("kept").0.wake_host();
    tokio::time::sleep(headers_timeout * 6 / 10).await;
    let (state, _conn) = rig.land(dialer).await;
    state.apply_settings(Some(5));
    let ready: Option<Acquired> = match waiter.await.expect("join").expect("acquired") {
        Acquisition::Ready(ready) => Some(ready),
        Acquisition::Spent => None,
    };
    let ready = ready.expect("the waiter goes on");
    assert!(ready.headers_deadline > tokio::time::Instant::now());
}

#[tokio::test]
async fn given_a_send_waiting_for_a_stream_while_a_dial_for_one_more_connection_is_open_then_headers_timeout_ends_the_wait()
 {
    let server = RawH2::start(|_| Policy {
        answer: Answer::Never,
        ..Policy::announcing(Announce::Limit(1))
    })
    .await;
    let endpoint = server.endpoint();
    let rig = Rig::timed(
        endpoint,
        Some(server),
        CONNECT_TIMEOUT,
        Duration::from_millis(300),
    );
    let (full, _held) = rig.land(rig.dialer()).await;
    full.apply_settings(Some(1));
    let _capacity = rig.dialer();
    let waiter = rig.waiter();
    let ended = tokio::time::timeout(Duration::from_secs(10), waiter)
        .await
        .expect("the wait for a stream has a deadline")
        .expect("join");
    let Outcome::Failed(error) = ended else {
        panic!("the waiter fails");
    };
    assert!(
        matches!(error, SendError::HeadersTimeout { .. }),
        "{error:?}"
    );
}
