use std::sync::Arc;
use std::time::Duration;

use tokio::time::{Instant, timeout, timeout_at};

use crate::client::Shared;
use crate::connect::{ConnectFailure, open};
use crate::deadline;
use crate::endpoint::Endpoint;
use crate::pool::{Dialer, Plan, PoolKey, WaitKind};
use crate::redact::Redactor;
use crate::send_error::SendError;
use crate::transport::Conn;

const READY_GRACE: Duration = Duration::from_millis(250);

pub(crate) async fn acquire(
    shared: &Arc<Shared>,
    endpoint: &Endpoint,
    key: &PoolKey,
    redactor: &Redactor,
    fresh: bool,
) -> Result<Conn, SendError> {
    let connect_limit = shared.config.connect_timeout;
    let connect_deadline = deadline::after(Instant::now(), connect_limit);
    let mut stream_deadline: Option<Instant> = None;
    loop {
        match shared.pool.plan(key, fresh) {
            Plan::Reuse(conn) => return Ok(conn),
            Plan::TryIdle(mut idle) => {
                if matches!(timeout(READY_GRACE, idle.sender.ready()).await, Ok(Ok(()))) {
                    idle.idle.disarm();
                    return Ok(Conn::H1(idle));
                }
            }
            Plan::Dial(dialer) => {
                let dialed = dial(shared, endpoint, dialer, connect_deadline, redactor).await?;
                if let Some(conn) = dialed {
                    return Ok(conn);
                }
            }
            Plan::Wait(mut changed, kind) => {
                let (at, failure) = match kind {
                    WaitKind::Dial => (
                        connect_deadline,
                        SendError::ConnectTimeout {
                            text: redactor.finish(&format!(
                                "no connection to {endpoint} within {connect_limit:?}"
                            )),
                        },
                    ),
                    WaitKind::Stream => {
                        let limit = shared.config.headers_timeout;
                        let at = *stream_deadline
                            .get_or_insert_with(|| deadline::after(Instant::now(), limit));
                        (
                            at,
                            SendError::HeadersTimeout {
                                text: redactor
                                    .finish(&format!("no free HTTP/2 stream within {limit:?}")),
                            },
                        )
                    }
                };
                if timeout_at(at, changed.changed()).await.is_err() {
                    return Err(failure);
                }
            }
        }
    }
}

async fn dial(
    shared: &Arc<Shared>,
    endpoint: &Endpoint,
    dialer: Dialer,
    connect_deadline: Instant,
    redactor: &Redactor,
) -> Result<Option<Conn>, SendError> {
    let limit = shared.config.connect_timeout;
    let opened = timeout_at(
        connect_deadline,
        open(&shared.tls, endpoint, &dialer.signal, shared.tuning),
    )
    .await;
    match opened {
        Err(_) => Err(SendError::ConnectTimeout {
            text: redactor.finish(&format!("no connection to {endpoint} within {limit:?}")),
        }),
        Ok(Err(ConnectFailure::Connect(text))) => Err(SendError::Connect {
            text: redactor.finish(&format!("{endpoint}: {text}")),
        }),
        Ok(Err(ConnectFailure::Tls(text))) => Err(SendError::Tls {
            text: redactor.finish(&format!("{endpoint}: {text}")),
        }),
        Ok(Ok(opened)) => Ok(dialer.register(opened)),
    }
}
