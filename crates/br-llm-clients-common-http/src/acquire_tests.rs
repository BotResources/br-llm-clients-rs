use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::acquire::{Acquisition, acquire};
use crate::client::{HttpClient, Shared};
use crate::connect::{Opened, Target, open};
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
use crate::turns_tests::within_turns;

pub(crate) type Acquiring = Pin<Box<dyn Future<Output = Result<Acquisition, SendError>> + Send>>;

pub(crate) const SETTINGS_WAIT: Duration = Duration::from_secs(60);
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) enum Outcome {
    Spent,
    Ready,
    Failed(SendError),
}

pub(crate) struct Rig {
    pub(crate) shared: Arc<Shared>,
    pub(crate) endpoint: Endpoint,
    pub(crate) key: PoolKey,
    server: Option<RawH2>,
}

impl Rig {
    pub(crate) async fn silent_server() -> Self {
        Self::slow_silent_server(Duration::ZERO, CONNECT_TIMEOUT).await
    }

    pub(crate) async fn slow_silent_server(
        accept_delay: Duration,
        connect_timeout: Duration,
    ) -> Self {
        let server = RawH2::start(move |_| Policy {
            answer: Answer::Never,
            accept_delay,
            ..Policy::announcing(Announce::Nothing)
        })
        .await;
        let endpoint = server.endpoint();
        Self::new(endpoint, Some(server), connect_timeout)
    }

    pub(crate) async fn announcing_server(limit: u32) -> Self {
        let server = RawH2::start(move |_| Policy {
            answer: Answer::Never,
            ..Policy::announcing(Announce::Limit(limit))
        })
        .await;
        let endpoint = server.endpoint();
        Self::new(endpoint, Some(server), CONNECT_TIMEOUT)
    }

    pub(crate) fn accepted(&self) -> usize {
        self.server.as_ref().map_or(0, RawH2::accepted)
    }

    pub(crate) fn dialing(&self) -> bool {
        self.shared
            .pool
            .lock()
            .get(&self.key)
            .is_some_and(|host| host.dialing)
    }

    pub(crate) fn acquisition(&self) -> Acquiring {
        let (shared, endpoint, key) = (
            Arc::clone(&self.shared),
            self.endpoint.clone(),
            self.key.clone(),
        );
        Box::pin(async move { acquire(&shared, &endpoint, &key, &Redactor::default()).await })
    }

    pub(crate) fn offline() -> Self {
        let endpoint = Endpoint::https("https://h.example/x").expect("endpoint");
        Self::new(endpoint, None, CONNECT_TIMEOUT)
    }

    fn new(endpoint: Endpoint, server: Option<RawH2>, connect_timeout: Duration) -> Self {
        Self::timed(endpoint, server, connect_timeout, config().headers_timeout)
    }

    pub(crate) fn timed(
        endpoint: Endpoint,
        server: Option<RawH2>,
        connect_timeout: Duration,
        headers_timeout: Duration,
    ) -> Self {
        let mut knobs = config();
        knobs.connect_timeout = connect_timeout;
        knobs.headers_timeout = headers_timeout;
        knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
        let client = HttpClient::new(knobs.build()).expect("client");
        Self {
            shared: Arc::clone(client.shared()),
            key: PoolKey::of(&endpoint),
            endpoint,
            server,
        }
    }

    pub(crate) fn dialer(&self) -> Dialer {
        match self.shared.pool.plan(&self.key, None) {
            Plan::Dial(dialer) => dialer,
            _ => panic!("the send dials"),
        }
    }

    pub(crate) fn waiter(&self) -> JoinHandle<Outcome> {
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

    fn parked(&self) -> usize {
        self.shared
            .pool
            .lock()
            .get(&self.key)
            .map_or(0, |host| host.waiting.load(Ordering::SeqCst))
    }

    pub(crate) async fn waiting(&self, count: usize) {
        within_turns(
            || self.parked() >= count,
            || {
                format!(
                    "{count} sends should be parked on the host and {} are: a send did not wait where it should, or a wake it needs never came",
                    self.parked()
                )
            },
        )
        .await;
    }

    pub(crate) async fn dial_started(&self) {
        within_turns(
            || self.dialing(),
            || "the send never started its dial".to_owned(),
        )
        .await;
    }

    pub(crate) async fn land(&self, dialer: Dialer) -> (Arc<H2State>, Conn) {
        let Ok(opened) = open(
            &self.shared.tls,
            &Target::of(&self.endpoint),
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

pub(crate) async fn outcome(waiter: JoinHandle<Outcome>) -> Outcome {
    tokio::time::timeout(Duration::from_secs(30), waiter)
        .await
        .expect("the waiter never ended: a wake it waits for is missing")
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
