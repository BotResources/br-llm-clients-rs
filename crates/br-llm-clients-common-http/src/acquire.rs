use std::sync::Arc;
use std::time::Duration;

use tokio::task::{JoinError, JoinHandle};
use tokio::time::{Instant, timeout, timeout_at};

use crate::claim::Claim;
use crate::client::Shared;
use crate::connect::{ConnectFailure, Target, open};
use crate::deadline;
use crate::dialer::{DialFailure, Dialer};
use crate::endpoint::Endpoint;
use crate::pool::{Plan, PoolKey, Wait, WaitKind};
use crate::redact::Redactor;
use crate::send_error::SendError;
use crate::server_fault::{about_endpoint, protocol_failure};
use crate::standing::Settled;
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
    headers_deadline: Option<Instant>,
}

impl Waiting<'_> {
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
        headers_deadline: None,
    };
    let mut _waiting: Option<LiveGuard> = None;
    let mut claim: Option<Claim> = None;
    loop {
        let held = claim.take();
        let plan = shared.pool.plan(key, held.as_ref());
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
            Plan::Settled(Settled::Spent) => return Ok(Acquisition::Spent),
            Plan::Settled(Settled::Broken(text)) => {
                return Err(protocol_failure(redactor, endpoint, &text));
            }
            Plan::Settled(Settled::Failed(failure)) => return Err(state.failure(failure)),
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
            Plan::Wait(Wait {
                mut changed,
                kind,
                guard,
            }) => {
                _waiting = Some(guard);
                match kind {
                    WaitKind::Dial(landing) => {
                        claim = Some(landing);
                        let _ = changed.changed().await;
                    }
                    WaitKind::Stream(next) => {
                        claim = next;
                        let limit = shared.config.headers_timeout;
                        let at = *state
                            .headers_deadline
                            .get_or_insert_with(|| deadline::after(Instant::now(), limit));
                        if timeout_at(at, changed.changed()).await.is_err() {
                            return Err(state.stream_timeout());
                        }
                    }
                }
            }
        }
    }
}

struct AbortOnDrop<T>(JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(crate) type DialOutcome = Result<Option<Conn>, DialFailure>;

pub(crate) fn joined(result: Result<DialOutcome, JoinError>) -> DialOutcome {
    result.unwrap_or_else(|_| Err(DialFailure::lost()))
}

async fn dial(state: &mut Waiting<'_>, dialer: Dialer) -> Result<Option<Conn>, SendError> {
    let mut task = AbortOnDrop(tokio::spawn(run_dial(
        Arc::clone(state.shared),
        Target::of(state.endpoint),
        dialer,
    )));
    joined((&mut task.0).await).map_err(|failure| state.failure(failure))
}

async fn run_dial(shared: Arc<Shared>, target: Target, dialer: Dialer) -> DialOutcome {
    let limit = shared.config.connect_timeout;
    let opened = timeout_at(
        deadline::after(Instant::now(), limit),
        open(&shared.tls, &target, &dialer.host, shared.tuning, limit),
    )
    .await;
    let failure = match opened {
        Err(_) => DialFailure::Timeout,
        Ok(Err(ConnectFailure::Connect(text))) => DialFailure::Connect(text),
        Ok(Err(ConnectFailure::Tls(text))) => DialFailure::Tls(text),
        Ok(Ok(opened)) => return Ok(dialer.register(opened)),
    };
    dialer.fail(failure.clone());
    Err(failure)
}
