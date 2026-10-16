use std::future::{Future, pending};
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;

use crate::classify::server_broke_http2;
use crate::h2state::H2State;
use crate::texts::hyper_text;

struct Gone(Arc<H2State>);

impl Drop for Gone {
    fn drop(&mut self) {
        self.0.retire();
        self.0.mark_gone();
        self.0.wake_host();
    }
}

pub(crate) async fn track_streams<C>(connection: C, state: Arc<H2State>, settings_wait: Duration)
where
    C: Future<Output = Result<(), hyper::Error>>,
{
    let _gone = Gone(Arc::clone(&state));
    let mut connection = pin!(connection);
    tokio::select! {
        ended = connection.as_mut() => {
            if let Err(error) = ended
                && server_broke_http2(&error)
            {
                state.record_violation(hyper_text(&error));
            }
        }
        () = settings_missing(&state, settings_wait) => {}
    }
}

async fn settings_missing(state: &H2State, wait: Duration) {
    sleep(wait).await;
    if !state.awaiting_settings() {
        pending::<()>().await;
    }
}

#[cfg(test)]
#[path = "h2track_tests.rs"]
mod tests;
