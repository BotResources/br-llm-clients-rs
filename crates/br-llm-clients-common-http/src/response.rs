use std::fmt;
use std::time::{Duration, SystemTime};

use crate::body::BodyStream;
use crate::headers::Headers;
use crate::retry_after;

pub struct Response {
    status: u16,
    headers: Headers,
    body: BodyStream,
}

impl Response {
    pub(crate) fn new(status: u16, headers: Headers, body: BodyStream) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn retry_after(&self) -> Option<Duration> {
        retry_after::parse(self.headers.get("retry-after")?, SystemTime::now())
    }

    pub fn retry_after_ms(&self) -> Option<Duration> {
        retry_after::parse_millis(self.headers.get("retry-after-ms")?)
    }

    pub fn body_mut(&mut self) -> &mut BodyStream {
        &mut self.body
    }

    pub fn into_parts(self) -> (u16, Headers, BodyStream) {
        (self.status, self.headers, self.body)
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("headers", &self.headers.len())
            .finish_non_exhaustive()
    }
}
