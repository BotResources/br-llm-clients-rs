use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::acquire::{Acquisition, acquire};
use crate::client::{HttpClient, Shared};
use crate::connect::{Opened, open};
use crate::dialer::Dialer;
use crate::endpoint::Endpoint;
use crate::h2state::H2State;
use crate::pool::{Plan, PoolKey};
use crate::redact::Redactor;
use crate::send_error::SendError;
use crate::support::config;
use crate::support::pki::Pki;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};
use crate::transport::Conn;

const SETTINGS_WAIT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

enum Outcome {
    Spent,
    Ready,
    Failed(SendError),
}

struct Rig {
    shared: Arc<Shared>,
    endpoint: Endpoint,
    key: PoolKey,
    _server: Option<RawH2>,
}

impl Rig {
    async fn silent_server() -> Self {
        Self::slow_silent_server(Duration::ZERO, CONNECT_TIMEOUT).await
    }

    async fn slow_silent_server(accept_delay: Duration, connect_timeout: Duration) -> Self {
        let server = RawH2::start(move |_| Policy {
            answer: Answer::Never,
            accept_delay,
            ..Policy::announcing(Announce::Nothing)
        })
        .await;
        let endpoint = server.endpoint();
        Self::new(endpoint, Some(server), connect_timeout)
    }

    fn offline() -> Self {
        let endpoint = Endpoint::https("https://h.example/x").expect("endpoint");
        Self::new(endpoint, None, CONNECT_TIMEOUT)
    }

    fn new(endpoint: Endpoint, server: Option<RawH2>, connect_timeout: Duration) -> Self {
        let mut knobs = config();
        knobs.connect_timeout = connect_timeout;
        knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
        let client = HttpClient::new(knobs.build()).expect("client");
        Self {
            shared: Arc::clone(client.shared()),
            key: PoolKey::of(&endpoint),
            endpoint,
            _server: server,
        }
    }

    fn dialer(&self) -> Dialer {
        match self.shared.pool.plan(&self.key, None) {
            Plan::Dial(dialer) => dialer,
            _ => panic!("the send dials"),
        }
    }

    fn waiter(&self) -> JoinHandle<Outcome> {
        let (shared, endpoint, key) = (
            Arc::clone(&self.shared),
            self.endpoint.clone(),
            self.key.clone(),
        );
        tokio::spawn(async move {
            match acquire(&shared, &endpoint, &key, &Redactor::default()).await {
                Ok(Acquisition::Spent) => Outcome::Spent,
                Ok(Acquisition::Ready(_)) => Outcome::Ready,
                Err(error) => Outcome::Failed(error),
            }
        })
    }

    async fn waiting(&self, count: usize) {
        loop {
            let waiting = self
                .shared
                .pool
                .lock()
                .get(&self.key)
                .map_or(0, |host| host.waiting.load(Ordering::SeqCst));
            if waiting >= count {
                return;
            }
            tokio::task::yield_now().await;
        }
    }

    async fn land(&self, dialer: Dialer) -> (Arc<H2State>, Conn) {
        let Ok(opened) = open(
            &self.shared.tls,
            &self.endpoint,
            &dialer.host,
            self.shared.tuning,
            SETTINGS_WAIT,
        )
        .await
        else {
            panic!("the connection opens");
        };
        let Opened::H2 { state, .. } = &opened else {
            panic!("the connection is HTTP/2");
        };
        let state = Arc::clone(state);
        let conn = dialer.register(opened).expect("registered");
        (state, conn)
    }
}

async fn outcome(waiter: JoinHandle<Outcome>) -> Outcome {
    tokio::time::timeout(Duration::from_secs(120), waiter)
        .await
        .expect("the waiter ends")
        .expect("join")
}

#[tokio::test]
async fn given_a_send_waiting_for_a_dial_when_the_connection_is_gone_before_it_wakes_then_it_spends_the_attempt()
 {
    let rig = Rig::silent_server().await;
    let dialer = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    let (state, _conn) = rig.land(dialer).await;
    state.apply_settings(Some(5));
    state.retire();
    state.mark_gone();
    assert!(matches!(outcome(waiter).await, Outcome::Spent));
}

#[tokio::test]
async fn given_a_send_waiting_for_a_dial_when_the_connection_announced_zero_then_it_spends_the_attempt()
 {
    let rig = Rig::silent_server().await;
    let dialer = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    let (state, _conn) = rig.land(dialer).await;
    state.apply_settings(Some(0));
    assert!(matches!(outcome(waiter).await, Outcome::Spent));
}

#[tokio::test]
async fn given_a_send_waiting_for_a_dial_when_the_connection_is_usable_then_it_goes_on_without_losing_an_attempt()
 {
    let rig = Rig::silent_server().await;
    let dialer = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    let (state, _conn) = rig.land(dialer).await;
    state.apply_settings(Some(5));
    assert!(matches!(outcome(waiter).await, Outcome::Ready));
}

#[tokio::test]
async fn given_a_send_waiting_for_a_dial_when_the_connection_broke_a_rule_then_it_fails_with_protocol()
 {
    let rig = Rig::silent_server().await;
    let dialer = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    let (state, _conn) = rig.land(dialer).await;
    state.record_violation("a rule was broken".to_owned());
    state.retire();
    let Outcome::Failed(error) = outcome(waiter).await else {
        panic!("the waiter fails");
    };
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_send_waiting_for_the_first_settings_when_they_announce_zero_then_it_spends_the_attempt()
 {
    let rig = Rig::silent_server().await;
    let (state, _conn) = rig.land(rig.dialer()).await;
    let waiter = rig.waiter();
    rig.waiting(1).await;
    state.apply_settings(Some(0));
    assert!(matches!(outcome(waiter).await, Outcome::Spent));
}

#[tokio::test]
async fn given_a_send_waiting_for_the_first_settings_when_the_connection_is_retired_then_it_spends_the_attempt()
 {
    let rig = Rig::silent_server().await;
    let (state, _conn) = rig.land(rig.dialer()).await;
    let waiter = rig.waiter();
    rig.waiting(1).await;
    state.retire();
    state.mark_gone();
    state.wake_host();
    assert!(matches!(outcome(waiter).await, Outcome::Spent));
}

#[tokio::test]
async fn given_a_send_waiting_for_a_stream_while_another_connection_is_dialed_when_that_one_is_gone_then_it_spends_the_attempt()
 {
    let rig = Rig::silent_server().await;
    let (full, _held) = rig.land(rig.dialer()).await;
    full.apply_settings(Some(1));
    let capacity = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    let (extra, _conn) = rig.land(capacity).await;
    extra.retire();
    extra.mark_gone();
    assert!(matches!(outcome(waiter).await, Outcome::Spent));
}

async fn failed_after(waiter: JoinHandle<Outcome>, elapsed: Duration) -> SendError {
    let started = Instant::now();
    let Outcome::Failed(error) = outcome(waiter).await else {
        panic!("the waiter fails");
    };
    assert_eq!(started.elapsed(), elapsed);
    error
}

#[tokio::test(start_paused = true)]
async fn given_a_send_waiting_behind_a_dial_that_never_ends_then_it_fails_with_connect_timeout_after_one_connect_timeout()
 {
    let rig = Rig::offline();
    let _dialer = rig.dialer();
    let waiter = rig.waiter();
    let error = failed_after(waiter, CONNECT_TIMEOUT).await;
    assert!(
        matches!(error, SendError::ConnectTimeout { .. }),
        "{error:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn given_a_send_waiting_behind_one_dial_and_then_behind_another_then_the_connect_timeout_restarts_for_the_second()
 {
    let rig = Rig::offline();
    let first = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    tokio::time::sleep(CONNECT_TIMEOUT / 2).await;
    drop(first);
    let _second = rig.dialer();
    let error = failed_after(waiter, CONNECT_TIMEOUT).await;
    assert!(
        matches!(error, SendError::ConnectTimeout { .. }),
        "{error:?}"
    );
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
async fn given_wakes_that_change_nothing_while_a_send_waits_behind_a_dial_then_its_connect_timeout_does_not_restart()
 {
    let rig = Rig::offline();
    let _dialer = rig.dialer();
    let waiter = rig.waiter();
    rig.waiting(1).await;
    for _ in 0..4 {
        tokio::time::sleep(CONNECT_TIMEOUT / 5).await;
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
    let error = failed_after(waiter, CONNECT_TIMEOUT / 5).await;
    assert!(
        matches!(error, SendError::ConnectTimeout { .. }),
        "{error:?}"
    );
}
