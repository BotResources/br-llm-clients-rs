use std::sync::Arc;
use std::time::Duration;

use tokio::time::{Instant, timeout, timeout_at};

use crate::client::Shared;
use crate::connect::{ConnectFailure, open};
use crate::deadline;
use crate::dialer::{DialFailure, Dialer};
use crate::endpoint::Endpoint;
use crate::pool::{Plan, PoolKey, WaitKind};
use crate::redact::Redactor;
use crate::send_error::SendError;
use crate::transport::Conn;

pub(crate) const READY_GRACE: Duration = Duration::from_millis(250);

pub(crate) struct Acquired {
    pub(crate) conn: Conn,
    pub(crate) headers_deadline: Instant,
}

struct Waiting<'a> {
    shared: &'a Arc<Shared>,
    endpoint: &'a Endpoint,
    redactor: &'a Redactor,
    connect_deadline: Instant,
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
                text: self.redactor.finish(&format!("{endpoint}: {text}")),
            },
            DialFailure::Tls(text) => SendError::Tls {
                text: self.redactor.finish(&format!("{endpoint}: {text}")),
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
) -> Result<Acquired, SendError> {
    let connect_deadline = deadline::after(Instant::now(), shared.config.connect_timeout);
    let mut state = Waiting {
        shared,
        endpoint,
        redactor,
        connect_deadline,
        headers_deadline: None,
    };
    let mut waited_from: Option<u64> = None;
    loop {
        match shared.pool.plan(key, waited_from) {
            Plan::Reuse(conn) => return Ok(state.ready(conn)),
            Plan::TryIdle(mut idle) => {
                if matches!(timeout(READY_GRACE, idle.sender.ready()).await, Ok(Ok(()))) {
                    idle.idle.disarm();
                    return Ok(state.ready(Conn::H1(idle)));
                }
            }
            Plan::Dial(dialer) => {
                if let Some(conn) = dial(&state, dialer).await? {
                    return Ok(state.ready(conn));
                }
            }
            Plan::Failed(failure) => return Err(state.failure(failure)),
            Plan::Wait(mut changed, kind, seq) => {
                let (at, timed_out) = match kind {
                    WaitKind::Dial => {
                        waited_from = Some(seq);
                        (state.connect_deadline, state.failure(DialFailure::Timeout))
                    }
                    WaitKind::Stream => {
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

async fn dial(state: &Waiting<'_>, dialer: Dialer) -> Result<Option<Conn>, SendError> {
    let shared = state.shared;
    let opened = timeout_at(
        state.connect_deadline,
        open(&shared.tls, state.endpoint, &dialer.host, shared.tuning),
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
