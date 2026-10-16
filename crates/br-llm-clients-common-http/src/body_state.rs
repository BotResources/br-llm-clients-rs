use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use hyper::body::Incoming;
use tokio::time::Sleep;

use crate::body_error::BodyError;
use crate::lease::Lease;
use crate::redact::Redactor;

pub(crate) struct Active {
    pub(crate) incoming: Incoming,
    pub(crate) lease: Lease,
    pub(crate) redactor: Arc<Redactor>,
    pub(crate) idle_limit: Duration,
    pub(crate) idle_timer: Option<Pin<Box<Sleep>>>,
    pub(crate) total_limit: Option<Duration>,
    pub(crate) total_timer: Option<Pin<Box<Sleep>>>,
    pub(crate) pending: Option<BodyError>,
}

pub(crate) enum State {
    Active(Box<Active>),
    Done,
}
