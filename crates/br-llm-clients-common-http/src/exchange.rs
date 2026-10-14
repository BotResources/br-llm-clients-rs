use std::sync::Arc;

use http::header::HOST;
use hyper::body::Incoming;
use tokio::time::{Instant, timeout};

use crate::body::{BodyParts, BodyStream};
use crate::classify::{Cause, classify};
use crate::client::Shared;
use crate::connect::{ConnectFailure, open};
use crate::encoding;
use crate::headers::Headers;
use crate::lease::Lease;
use crate::payload::Payload;
use crate::pool::PoolKey;
use crate::prepare::Prepared;
use crate::redact::Redactor;
use crate::response::Response;
use crate::send_error::SendError;
use crate::transport::{Conn, H1Conn};

const MAX_STALE_RETRIES: usize = 3;

struct Acquired {
    conn: Conn,
    reused: bool,
}

enum Attempt {
    Done(Box<Response>),
    NotSent,
    Failed(SendError),
}

pub(crate) async fn exchange(
    shared: &Arc<Shared>,
    prepared: &Prepared,
    redactor: &Arc<Redactor>,
    total_deadline: Option<Instant>,
) -> Result<Response, SendError> {
    let key = PoolKey::of(&prepared.endpoint);
    let mut stale = 0usize;
    loop {
        let acquired = acquire(shared, prepared, &key, redactor).await?;
        let reused = acquired.reused;
        match attempt(shared, prepared, &key, redactor, total_deadline, acquired).await {
            Attempt::Done(response) => return Ok(*response),
            Attempt::NotSent if reused && stale < MAX_STALE_RETRIES => {
                stale = stale.saturating_add(1);
            }
            Attempt::NotSent => {
                return Err(SendError::Interrupted {
                    text: redactor.clean("the connection closed before the request was written"),
                });
            }
            Attempt::Failed(error) => return Err(error),
        }
    }
}

async fn acquire(
    shared: &Arc<Shared>,
    prepared: &Prepared,
    key: &PoolKey,
    redactor: &Redactor,
) -> Result<Acquired, SendError> {
    if let Some(conn) = shared.pool.checkout(key).await {
        return Ok(Acquired { conn, reused: true });
    }
    let limit = shared.config.connect_timeout;
    let opened = timeout(limit, open(&shared.tls, &prepared.endpoint, redactor)).await;
    let endpoint = &prepared.endpoint;
    match opened {
        Err(_) => Err(SendError::ConnectTimeout {
            text: redactor.clean(&format!("no connection to {endpoint} within {limit:?}")),
        }),
        Ok(Err(ConnectFailure::Connect(text))) => Err(SendError::Connect {
            text: redactor.clean(&format!("{endpoint}: {text}")),
        }),
        Ok(Err(ConnectFailure::Tls(text))) => Err(SendError::Tls {
            text: redactor.clean(&format!("{endpoint}: {text}")),
        }),
        Ok(Err(ConnectFailure::Handshake(text))) => Err(SendError::Interrupted {
            text: redactor.clean(&format!("{endpoint}: {text}")),
        }),
        Ok(Ok(conn)) => {
            if let Conn::H2(sender) = &conn {
                shared.pool.offer_h2(key, sender);
            }
            Ok(Acquired {
                conn,
                reused: false,
            })
        }
    }
}

fn build_request(conn: &Conn, prepared: &Prepared) -> Result<http::Request<Payload>, SendError> {
    let h2 = matches!(conn, Conn::H2(_));
    let uri = if h2 {
        prepared.endpoint.absolute_uri()
    } else {
        prepared.endpoint.origin_uri()
    }
    .map_err(|_| SendError::Protocol {
        text: "the endpoint cannot be sent".to_owned(),
    })?;
    let mut request = http::Request::new(Payload::new(prepared.body.clone()));
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
    let request = match build_request(&acquired.conn, prepared) {
        Ok(request) => request,
        Err(error) => return Attempt::Failed(error),
    };
    let limit = shared.config.headers_timeout;
    let sent = match acquired.conn {
        Conn::H1(mut conn) => {
            let outcome = timeout(limit, send_h1(&mut conn, request)).await;
            outcome.map(|result| {
                result.map(|response| {
                    (
                        response,
                        Lease::Exclusive {
                            conn,
                            pool: Arc::clone(&shared.pool),
                            key: key.clone(),
                        },
                    )
                })
            })
        }
        Conn::H2(mut sender) => {
            let outcome = timeout(limit, send_h2(&mut sender, request)).await;
            outcome.map(|result| result.map(|response| (response, Lease::Shared)))
        }
    };
    match sent {
        Err(_) => Attempt::Failed(SendError::HeadersTimeout {
            text: redactor.clean(&format!("no response headers within {limit:?}")),
        }),
        Ok(Err(Unsent::NotSent)) => Attempt::NotSent,
        Ok(Err(Unsent::Failed(error))) => Attempt::Failed(send_failure(&error, redactor)),
        Ok(Ok((response, lease))) => Attempt::Done(Box::new(into_response(
            shared,
            response,
            lease,
            redactor,
            total_deadline,
        ))),
    }
}

enum Unsent {
    NotSent,
    Failed(hyper::Error),
}

async fn send_h1(
    conn: &mut H1Conn,
    request: http::Request<Payload>,
) -> Result<http::Response<Incoming>, Unsent> {
    if conn.sender.ready().await.is_err() {
        return Err(Unsent::NotSent);
    }
    conn.sender
        .try_send_request(request)
        .await
        .map_err(into_unsent)
}

async fn send_h2(
    sender: &mut crate::transport::H2Sender,
    request: http::Request<Payload>,
) -> Result<http::Response<Incoming>, Unsent> {
    sender.try_send_request(request).await.map_err(into_unsent)
}

fn into_unsent(error: hyper::client::conn::TrySendError<http::Request<Payload>>) -> Unsent {
    if error.message().is_some() {
        Unsent::NotSent
    } else {
        Unsent::Failed(error.into_error())
    }
}

fn send_failure(error: &hyper::Error, redactor: &Redactor) -> SendError {
    let text = redactor.describe(error);
    match classify(error) {
        Cause::Interrupted => SendError::Interrupted { text },
        Cause::Protocol => SendError::Protocol { text },
    }
}

fn into_response(
    shared: &Arc<Shared>,
    response: http::Response<Incoming>,
    lease: Lease,
    redactor: &Arc<Redactor>,
    total_deadline: Option<Instant>,
) -> Response {
    let (parts, incoming) = response.into_parts();
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
    Response::new(parts.status.as_u16(), Headers::new(parts.headers), body)
}
