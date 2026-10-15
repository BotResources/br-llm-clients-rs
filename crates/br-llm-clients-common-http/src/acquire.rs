use std::sync::Arc;
use std::time::Duration;

use tokio::time::{Instant, timeout, timeout_at};

use crate::claim::{Claim, Landing};
use crate::client::Shared;
use crate::connect::{ConnectFailure, open};
use crate::deadline;
use crate::dialer::{DialFailure, Dialer};
use crate::endpoint::Endpoint;
use crate::h2state::Standing;
use crate::pool::{Plan, PoolKey, Wait, WaitKind};
use crate::redact::Redactor;
use crate::send_error::SendError;
use crate::server_fault::{about_endpoint, protocol_failure};
use crate::transport::{Conn, LiveGuard};

pub(crate) const READY_GRACE: Duration = Duration::from_millis(250);

pub(crate) enum Acquisition {
    Ready(Acquired),
    Spent,
}

pub(crate) struct Acquired {
    pub(crate) conn: Conn,
    pub(crate) headers_deadline: Instant,
}

struct Waiting<'a> {
    shared: &'a Arc<Shared>,
    endpoint: &'a Endpoint,
    redactor: &'a Redactor,
    dial_wait: Option<(Arc<Landing>, Instant)>,
    headers_deadline: Option<Instant>,
}

impl Waiting<'_> {
    fn dial_wait_deadline(&mut self, landing: &Arc<Landing>) -> Instant {
        if let Some((seen, at)) = &self.dial_wait
            && Arc::ptr_eq(seen, landing)
        {
            return *at;
        }
        let at = deadline::after(Instant::now(), self.shared.config.connect_timeout);
        self.dial_wait = Some((Arc::clone(landing), at));
        at
    }

    fn ready(&mut self, conn: Conn) -> Acquired {
        let limit = self.shared.config.headers_timeout;
        let deadline = *self
            .headers_deadline
            .get_or_insert_with(|| deadline::after(Instant::now(), limit));
        Acquired {
            conn,
            headers_deadline: deadline,
        }
    }

    fn failure(&self, failure: DialFailure) -> SendError {
        let endpoint = self.endpoint;
        let limit = self.shared.config.connect_timeout;
        match failure {
            DialFailure::Timeout => SendError::ConnectTimeout {
                text: self
                    .redactor
                    .finish(&format!("no connection to {endpoint} within {limit:?}")),
            },
            DialFailure::Connect(text) => SendError::Connect {
                text: about_endpoint(self.redactor, endpoint, &text),
            },
            DialFailure::Tls(text) => SendError::Tls {
                text: about_endpoint(self.redactor, endpoint, &text),
            },
        }
    }

    fn stream_timeout(&self) -> SendError {
        let limit = self.shared.config.headers_timeout;
        SendError::HeadersTimeout {
            text: self
                .redactor
                .finish(&format!("no free HTTP/2 stream within {limit:?}")),
        }
    }
}

pub(crate) async fn acquire(
    shared: &Arc<Shared>,
    endpoint: &Endpoint,
    key: &PoolKey,
    redactor: &Redactor,
) -> Result<Acquisition, SendError> {
    let mut state = Waiting {
        shared,
        endpoint,
        redactor,
        dial_wait: None,
        headers_deadline: None,
    };
    let mut waited_from: Option<u64> = None;
    let mut _waiting: Option<LiveGuard> = None;
    let mut claim: Option<Claim> = None;
    loop {
        if let Some(held) = claim.take() {
            match held.standing() {
                Standing::Spent => return Ok(Acquisition::Spent),
                Standing::Broken(text) => return Err(protocol_failure(redactor, endpoint, &text)),
                Standing::Open => {}
            }
        }
        let plan = shared.pool.plan(key, waited_from);
        _waiting = None;
        if matches!(
            plan,
            Plan::Dial(_)
                | Plan::Wait(Wait {
                    kind: WaitKind::Dial(_),
                    ..
                })
        ) {
            state.headers_deadline = None;
        }
        match plan {
            Plan::Reuse(conn) => return Ok(Acquisition::Ready(state.ready(conn))),
            Plan::TryIdle(mut idle) => {
                let ready = matches!(timeout(READY_GRACE, idle.sender.ready()).await, Ok(Ok(())));
                if ready && idle.idle.disarm() {
                    return Ok(Acquisition::Ready(state.ready(Conn::H1(idle))));
                }
            }
            Plan::Dial(dialer) => {
                if let Some(conn) = dial(&mut state, dialer).await? {
                    return Ok(Acquisition::Ready(state.ready(conn)));
                }
            }
            Plan::Failed(failure) => return Err(state.failure(failure)),
            Plan::Wait(Wait {
                mut changed,
                kind,
                seq,
                guard,
                claim: held,
            }) => {
                _waiting = Some(guard);
                let (at, timed_out) = match kind {
                    WaitKind::Dial(landing) => {
                        waited_from = Some(seq);
                        let at = state.dial_wait_deadline(&landing);
                        claim = Some(Claim::Landing(landing));
                        (at, state.failure(DialFailure::Timeout))
                    }
                    WaitKind::Stream => {
                        claim = held;
                        let limit = shared.config.headers_timeout;
                        let at = *state
                            .headers_deadline
                            .get_or_insert_with(|| deadline::after(Instant::now(), limit));
                        (at, state.stream_timeout())
                    }
                };
                if timeout_at(at, changed.changed()).await.is_err() {
                    return Err(timed_out);
                }
            }
        }
    }
}

async fn dial(state: &mut Waiting<'_>, dialer: Dialer) -> Result<Option<Conn>, SendError> {
    let shared = state.shared;
    let limit = shared.config.connect_timeout;
    let opened = timeout_at(
        deadline::after(Instant::now(), limit),
        open(
            &shared.tls,
            state.endpoint,
            &dialer.host,
            shared.tuning,
            limit,
        ),
    )
    .await;
    let failure = match opened {
        Err(_) => DialFailure::Timeout,
        Ok(Err(ConnectFailure::Connect(text))) => DialFailure::Connect(text),
        Ok(Err(ConnectFailure::Tls(text))) => DialFailure::Tls(text),
        Ok(Ok(opened)) => return Ok(dialer.register(opened)),
    };
    let error = state.failure(failure.clone());
    dialer.fail(failure);
    Err(error)
}
