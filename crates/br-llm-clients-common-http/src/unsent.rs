use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use hyper::body::Incoming;
use hyper::client::conn::TrySendError;
use tokio::sync::watch;

use crate::classify::classify;
use crate::h2state::Permit;
use crate::payload::Payload;
use crate::transport::{H1Conn, H2Sender};

pub(crate) enum Unsent {
    Unprocessed { taken: bool, retire: bool },
    Failed(hyper::Error, bool),
    Violation(String),
}

pub(crate) async fn send_h1(
    conn: &mut H1Conn,
    request: http::Request<Payload>,
    taken: &Arc<AtomicBool>,
) -> Result<http::Response<Incoming>, Unsent> {
    if conn.sender.ready().await.is_err() {
        return Err(Unsent::Unprocessed {
            taken: false,
            retire: true,
        });
    }
    conn.sender
        .try_send_request(request)
        .await
        .map_err(|error| into_unsent(error, taken))
}

pub(crate) async fn send_h2(
    sender: &mut H2Sender,
    permit: &Permit,
    request: http::Request<Payload>,
    taken: &Arc<AtomicBool>,
) -> Result<http::Response<Incoming>, Unsent> {
    if permit.announced_limit().await == 0 {
        return Err(Unsent::Unprocessed {
            taken: false,
            retire: false,
        });
    }
    sender
        .try_send_request(request)
        .await
        .map_err(|error| into_unsent(error, taken))
}

pub(crate) async fn never_taken(gone: &mut watch::Receiver<bool>, taken: &Arc<AtomicBool>) {
    if gone.wait_for(|ended| *ended).await.is_ok() && !taken.load(Ordering::SeqCst) {
        return;
    }
    std::future::pending::<()>().await;
}

pub(crate) fn must_retire(unsent: &Unsent) -> bool {
    match unsent {
        Unsent::Unprocessed { retire, .. } => *retire,
        Unsent::Failed(error, taken) => classify(error, *taken, true).retires_connection(),
        Unsent::Violation(_) => true,
    }
}

fn into_unsent(error: TrySendError<http::Request<Payload>>, taken: &Arc<AtomicBool>) -> Unsent {
    if error.message().is_some() {
        Unsent::Unprocessed {
            taken: false,
            retire: true,
        }
    } else {
        Unsent::Failed(error.into_error(), taken.load(Ordering::SeqCst))
    }
}
