use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use http::header::HOST;
use hyper::body::Incoming;
use tokio::time::{Instant, timeout_at};

use crate::acquire::{Acquired, Acquisition, acquire};
use crate::body::{BodyParts, BodyStream};
use crate::classify::{Cause, classify};
use crate::client::Shared;
use crate::encoding;
use crate::headers::Headers;
use crate::lease::Lease;
use crate::limits::{MAX_ATTEMPTS, MAX_HEADER_LINES};
use crate::payload::Payload;
use crate::pool::PoolKey;
use crate::prepare::Prepared;
use crate::redact::Redactor;
use crate::response::Response;
use crate::send_error::SendError;
use crate::server_fault::protocol_failure;
use crate::texts::hyper_text;
use crate::transport::Conn;
use crate::unsent::{Unsent, must_retire, never_taken, send_h1, send_h2};

enum Attempt {
    Done(Box<Response>),
    Unprocessed { taken: bool },
    Failed(SendError),
}

pub(crate) async fn exchange(
    shared: &Arc<Shared>,
    prepared: &Prepared,
    redactor: &Arc<Redactor>,
    total_deadline: Option<Instant>,
) -> Result<Response, SendError> {
    let key = PoolKey::of(&prepared.endpoint);
    let mut any_taken = false;
    for _ in 0..MAX_ATTEMPTS {
        let acquired = match acquire(shared, &prepared.endpoint, &key, redactor).await? {
            Acquisition::Ready(acquired) => acquired,
            Acquisition::Spent => continue,
        };
        match attempt(shared, prepared, &key, redactor, total_deadline, acquired).await {
            Attempt::Done(response) => return Ok(*response),
            Attempt::Unprocessed { taken } => any_taken |= taken,
            Attempt::Failed(error) => return Err(error),
        }
    }
    Err(if any_taken {
        SendError::Interrupted {
            text: redactor.finish("the server did not process the request on any of the attempts"),
        }
    } else {
        SendError::Connect {
            text: redactor.finish("no connection could carry the request; none of it was written"),
        }
    })
}

fn build_request(
    conn: &Conn,
    prepared: &Prepared,
    taken: &Arc<AtomicBool>,
    redactor: &Redactor,
) -> Result<http::Request<Payload>, SendError> {
    let h2 = matches!(conn, Conn::H2 { .. });
    let uri = if h2 {
        prepared.endpoint.absolute_uri()
    } else {
        prepared.endpoint.origin_uri()
    }
    .map_err(|_| protocol_failure(redactor, &prepared.endpoint, "the endpoint cannot be sent"))?;
    let payload = Payload::new(prepared.body.clone(), Arc::clone(taken));
    let mut request = http::Request::new(payload);
    *request.method_mut() = http::Method::POST;
    *request.uri_mut() = uri;
    for (name, value) in &prepared.headers {
        if h2 && *name == HOST {
            continue;
        }
        request.headers_mut().append(name.clone(), value.clone());
    }
    Ok(request)
}

async fn attempt(
    shared: &Arc<Shared>,
    prepared: &Prepared,
    key: &PoolKey,
    redactor: &Arc<Redactor>,
    total_deadline: Option<Instant>,
    acquired: Acquired,
) -> Attempt {
    let Acquired {
        conn,
        headers_deadline,
    } = acquired;
    let taken = Arc::new(AtomicBool::new(false));
    let request = match build_request(&conn, prepared, &taken, redactor) {
        Ok(request) => request,
        Err(error) => return Attempt::Failed(error),
    };
    let limit = shared.config.headers_timeout;
    let h2 = matches!(conn, Conn::H2 { .. });
    let sent = match conn {
        Conn::H1(mut conn) => {
            let outcome = timeout_at(headers_deadline, send_h1(&mut conn, request, &taken)).await;
            outcome.map(|result| {
                result.map(|response| {
                    let lease = Lease::Exclusive {
                        conn,
                        pool: Arc::clone(&shared.pool),
                        key: key.clone(),
                    };
                    (response, lease)
                })
            })
        }
        Conn::H2 { mut sender, permit } => {
            let mut gone = permit.gone();
            let outcome = timeout_at(headers_deadline, async {
                tokio::select! {
                    biased;
                    result = send_h2(&mut sender, &permit, request, &taken) => result,
                    () = never_taken(&mut gone, &taken) => {
                        Err(Unsent::Unprocessed {
                            taken: false,
                            retire: true,
                        })
                    }
                }
            })
            .await;
            match outcome {
                Ok(Ok(response)) => Ok(Ok((response, Lease::Shared(permit)))),
                Ok(Err(unsent)) => {
                    let unsent = match permit.violation() {
                        Some(text) => Unsent::Violation(text),
                        None => unsent,
                    };
                    if must_retire(&unsent) {
                        permit.retire_connection();
                    }
                    Ok(Err(unsent))
                }
                Err(elapsed) => Err(elapsed),
            }
        }
    };
    match sent {
        Err(_) => Attempt::Failed(SendError::HeadersTimeout {
            text: redactor.finish(&format!("no response headers within {limit:?}")),
        }),
        Ok(Err(Unsent::Unprocessed { taken, .. })) => Attempt::Unprocessed { taken },
        Ok(Err(Unsent::Violation(text))) => {
            Attempt::Failed(protocol_failure(redactor, &prepared.endpoint, &text))
        }
        Ok(Err(Unsent::Failed(error, taken))) => match classify(&error, taken, h2) {
            Cause::Unprocessed { taken, .. } => Attempt::Unprocessed { taken },
            Cause::Interrupted { .. } => Attempt::Failed(SendError::Interrupted {
                text: redactor.finish(&hyper_text(&error)),
            }),
            Cause::Protocol => Attempt::Failed(protocol_failure(
                redactor,
                &prepared.endpoint,
                &hyper_text(&error),
            )),
        },
        Ok(Ok((response, lease))) => {
            into_response(shared, prepared, response, lease, redactor, total_deadline)
        }
    }
}

fn into_response(
    shared: &Arc<Shared>,
    prepared: &Prepared,
    response: http::Response<Incoming>,
    lease: Lease,
    redactor: &Arc<Redactor>,
    total_deadline: Option<Instant>,
) -> Attempt {
    let (parts, incoming) = response.into_parts();
    if parts.headers.len() > MAX_HEADER_LINES {
        return Attempt::Failed(protocol_failure(
            redactor,
            &prepared.endpoint,
            "the response has more than 1024 header lines",
        ));
    }
    let unsupported_encoding = encoding::unsupported(&parts.headers);
    let body = BodyStream::new(BodyParts {
        incoming,
        lease,
        redactor: Arc::clone(redactor),
        idle_limit: shared.config.idle_timeout,
        total_deadline,
        total_limit: shared.config.total_timeout,
        unsupported_encoding,
        bodiless: matches!(parts.status.as_u16(), 204 | 304),
    });
    Attempt::Done(Box::new(Response::new(
        parts.status.as_u16(),
        Headers::new(parts.headers),
        body,
    )))
}
