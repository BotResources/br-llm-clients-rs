use std::future::{Future, pending};
use std::pin::pin;

use tokio::sync::watch;
use tokio::time::{Instant, sleep_until};

pub(crate) async fn drive<F: Future>(connection: F, mut idle: watch::Receiver<Option<Instant>>) {
    let mut connection = pin!(connection);
    loop {
        let expiry = *idle.borrow_and_update();
        tokio::select! {
            _ = connection.as_mut() => return,
            changed = idle.changed() => {
                if changed.is_err() {
                    connection.await;
                    return;
                }
            }
            () = until(expiry) => return,
        }
    }
}

async fn until(at: Option<Instant>) {
    match at {
        Some(at) => sleep_until(at).await,
        None => pending::<()>().await,
    }
}
