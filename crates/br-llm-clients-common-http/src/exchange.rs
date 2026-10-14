use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use http::header::HOST;
use hyper::body::Incoming;
use hyper::client::conn::TrySendError;
use tokio::time::{Instant, timeout};

use crate::acquire::acquire;
use crate::body::{BodyParts, BodyStream};
use crate::classify::{Cause, classify};
use crate::client::Shared;
use crate::encoding;
use crate::headers::Headers;
use crate::lease::Lease;
use crate::payload::Payload;
use crate::pool::PoolKey;
use crate::prepare::Prepared;
use crate::redact::Redactor;
use crate::response::Response;
use crate::send_error::SendError;
use crate::texts::hyper_text;
use crate::transport::{Conn, H1Conn, H2Sender};

pub(crate) const MAX_RESPONSE_HEADER_LINES: usize = 1024;

enum Attempt {
    Done(Box<Response>),
    Unprocessed { written: bool },
    Failed(SendError),
}

enum Unsent {
    Unprocessed { written: bool },
    Failed(hyper::Error, bool),
}

pub(crate) async fn exchange(
    shared: &Arc<Shared>,
    prepared: &Prepared,
    redactor: &Arc<Redactor>,
    total_deadline: Option<Instant>,
) -> Result<Response, SendError> {
    let key = PoolKey::of(&prepared.endpoint);
    let mut last = false;
    for fresh in [false, true, true] {
        let conn = acquire(shared, &prepared.endpoint, &key, redactor, fresh).await?;
        match attempt(shared, prepared, &key, redactor, total_deadline, conn).await {
            Attempt::Done(response) => return Ok(*response),
            Attempt::Unprocessed { written } => last = written,
            Attempt::Failed(error) => return Err(error),
        }
    }
    Err(if last {
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
) -> Result<http::Request<Payload>, SendError> {
    let h2 = matches!(conn, Conn::H2 { .. });
    let uri = if h2 {
        prepared.endpoint.absolute_uri()
    } else {
        prepared.endpoint.origin_uri()
    }
    .map_err(|_| SendError::Protocol {
        text: "the endpoint cannot be sent".to_owned(),
    })?;
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
    conn: Conn,
) -> Attempt {
    let taken = Arc::new(AtomicBool::new(false));
    let request = match build_request(&conn, prepared, &taken) {
        Ok(request) => request,
        Err(error) => return Attempt::Failed(error),
    };
    let limit = shared.config.headers_timeout;
    let sent = match conn {
        Conn::H1(mut conn) => {
            let outcome = timeout(limit, send_h1(&mut conn, request, &taken)).await;
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
            let outcome = timeout(limit, send_h2(&mut sender, request, &taken)).await;
            outcome.map(|result| result.map(|response| (response, Lease::Shared(permit))))
        }
    };
    match sent {
        Err(_) => Attempt::Failed(SendError::HeadersTimeout {
            text: redactor.finish(&format!("no response headers within {limit:?}")),
        }),
        Ok(Err(Unsent::Unprocessed { written })) => Attempt::Unprocessed { written },
        Ok(Err(Unsent::Failed(error, taken))) => match classify(&error, taken) {
            Cause::Unprocessed { written } => Attempt::Unprocessed { written },
            Cause::Interrupted => Attempt::Failed(SendError::Interrupted {
                text: redactor.finish(&hyper_text(&error)),
            }),
            Cause::Protocol => Attempt::Failed(SendError::Protocol {
                text: redactor.finish(&hyper_text(&error)),
            }),
        },
        Ok(Ok((response, lease))) => {
            into_response(shared, response, lease, redactor, total_deadline)
        }
    }
}

async fn send_h1(
    conn: &mut H1Conn,
    request: http::Request<Payload>,
    taken: &Arc<AtomicBool>,
) -> Result<http::Response<Incoming>, Unsent> {
    if conn.sender.ready().await.is_err() {
        return Err(Unsent::Unprocessed { written: false });
    }
    conn.sender
        .try_send_request(request)
        .await
        .map_err(|error| into_unsent(error, taken))
}

async fn send_h2(
    sender: &mut H2Sender,
    request: http::Request<Payload>,
    taken: &Arc<AtomicBool>,
) -> Result<http::Response<Incoming>, Unsent> {
    sender
        .try_send_request(request)
        .await
        .map_err(|error| into_unsent(error, taken))
}

fn into_unsent(error: TrySendError<http::Request<Payload>>, taken: &Arc<AtomicBool>) -> Unsent {
    if error.message().is_some() {
        Unsent::Unprocessed { written: false }
    } else {
        Unsent::Failed(error.into_error(), taken.load(Ordering::SeqCst))
    }
}

fn into_response(
    shared: &Arc<Shared>,
    response: http::Response<Incoming>,
    lease: Lease,
    redactor: &Arc<Redactor>,
    total_deadline: Option<Instant>,
) -> Attempt {
    let (parts, incoming) = response.into_parts();
    if parts.headers.len() > MAX_RESPONSE_HEADER_LINES {
        return Attempt::Failed(SendError::Protocol {
            text: redactor.finish("the response has more than 1024 header lines"),
        });
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
    });
    Attempt::Done(Box::new(Response::new(
        parts.status.as_u16(),
        Headers::new(parts.headers),
        body,
    )))
}
