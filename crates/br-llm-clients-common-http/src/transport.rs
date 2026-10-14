use hyper::client::conn::{http1, http2};
use tokio::task::AbortHandle;

use crate::payload::Payload;

pub(crate) type H1Sender = http1::SendRequest<Payload>;
pub(crate) type H2Sender = http2::SendRequest<Payload>;

pub(crate) struct H1Conn {
    pub(crate) sender: H1Sender,
    driver: AbortHandle,
}

impl H1Conn {
    pub(crate) fn new(sender: H1Sender, driver: AbortHandle) -> Self {
        Self { sender, driver }
    }
}

impl Drop for H1Conn {
    fn drop(&mut self) {
        self.driver.abort();
    }
}

pub(crate) enum Conn {
    H1(H1Conn),
    H2(H2Sender),
}
