use std::future::{Future, pending};
use std::pin::pin;

use tokio::time::{Instant, sleep_until};

use crate::idle::IdleWatch;

pub(crate) async fn drive<F: Future>(connection: F, mut idle: IdleWatch) {
    let mut connection = pin!(connection);
    loop {
        let armed_until = idle.armed_until();
        tokio::select! {
            _ = connection.as_mut() => return,
            () = idle.changed() => {}
            () = until(armed_until) => {
                if armed_until.is_some_and(|at| idle.expire(at)) {
                    return;
                }
            }
        }
    }
}

async fn until(at: Option<Instant>) {
    match at {
        Some(at) => sleep_until(at).await,
        None => pending::<()>().await,
    }
}
