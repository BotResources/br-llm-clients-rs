use std::future::pending;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;

use crate::h2state::H2State;

struct Gone(Arc<H2State>);

impl Drop for Gone {
    fn drop(&mut self) {
        self.0.mark_gone();
        self.0.retire();
        self.0.wake_host();
    }
}

pub(crate) async fn track_streams<C: std::future::Future>(
    connection: C,
    state: Arc<H2State>,
    settings_wait: Duration,
) {
    let _gone = Gone(Arc::clone(&state));
    tokio::select! {
        _ = connection => {}
        () = settings_missing(&state, settings_wait) => {}
    }
}

async fn settings_missing(state: &H2State, wait: Duration) {
    sleep(wait).await;
    if !state.awaiting_settings() {
        pending::<()>().await;
    }
}
