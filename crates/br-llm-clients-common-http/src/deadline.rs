use std::time::Duration;

use tokio::time::Instant;

const FAR: Duration = Duration::from_secs(60 * 60 * 24 * 365);

pub(crate) fn after(start: Instant, span: Duration) -> Instant {
    start
        .checked_add(span)
        .or_else(|| start.checked_add(FAR))
        .unwrap_or(start)
}
