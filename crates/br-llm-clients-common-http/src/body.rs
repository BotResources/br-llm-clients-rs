use std::fmt;
use std::future::poll_fn;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use hyper::body::{Body, Incoming};
use tokio::time::{Instant, sleep, sleep_until};

use crate::body_error::BodyError;
use crate::body_state::{Active, State};
use crate::classify::{Cause, classify};
use crate::lease::Lease;
use crate::redact::Redactor;

pub struct BodyStream {
    state: State,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Collected {
    pub bytes: Vec<u8>,
    pub cut: bool,
}

pub(crate) struct BodyParts {
    pub(crate) incoming: Incoming,
    pub(crate) lease: Lease,
    pub(crate) redactor: Arc<Redactor>,
    pub(crate) idle_limit: Duration,
    pub(crate) total_deadline: Option<Instant>,
    pub(crate) total_limit: Option<Duration>,
    pub(crate) unsupported_encoding: Option<BodyError>,
}

impl BodyStream {
    pub(crate) fn new(parts: BodyParts) -> Self {
        if parts.unsupported_encoding.is_none() && parts.incoming.is_end_stream() {
            parts.lease.finish_clean();
            return Self { state: State::Done };
        }
        let pending = parts
            .unsupported_encoding
            .map(|e| scrubbed(e, &parts.redactor));
        let active = Active {
            incoming: parts.incoming,
            lease: parts.lease,
            redactor: parts.redactor,
            idle_limit: parts.idle_limit,
            idle_timer: None,
            total_limit: parts.total_limit,
            total_timer: parts.total_deadline.map(|d| Box::pin(sleep_until(d))),
            pending,
        };
        Self {
            state: State::Active(Box::new(active)),
        }
    }

    pub async fn next_chunk(&mut self) -> Result<Option<Bytes>, BodyError> {
        poll_fn(|cx| self.poll_next_chunk(cx)).await
    }

    pub async fn collect(&mut self, limit: usize) -> Result<Collected, BodyError> {
        let mut bytes: Vec<u8> = Vec::new();
        while let Some(chunk) = self.next_chunk().await? {
            let room = limit.saturating_sub(bytes.len());
            if chunk.len() > room {
                bytes.extend_from_slice(chunk.get(..room).unwrap_or_default());
                return Ok(Collected { bytes, cut: true });
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(Collected { bytes, cut: false })
    }

    pub fn poll_next_chunk(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<Bytes>, BodyError>> {
        loop {
            let State::Active(active) = &mut self.state else {
                return Poll::Ready(Ok(None));
            };
            if let Some(error) = active.pending.take() {
                self.state = State::Done;
                return Poll::Ready(Err(error));
            }
            if poll_deadline(&mut active.total_timer, cx) {
                let text = active.redactor.clean(&format!(
                    "the response did not end within the total timeout of {:?}",
                    active.total_limit.unwrap_or_default()
                ));
                self.state = State::Done;
                return Poll::Ready(Err(BodyError::TotalTimeout { text }));
            }
            match Pin::new(&mut active.incoming).poll_frame(cx) {
                Poll::Ready(Some(Ok(frame))) => match frame.into_data() {
                    Ok(data) if !data.is_empty() => {
                        active.idle_timer = None;
                        return Poll::Ready(Ok(Some(data)));
                    }
                    _ => continue,
                },
                Poll::Ready(Some(Err(error))) => {
                    let failure = body_failure(&error, &active.redactor);
                    self.state = State::Done;
                    return Poll::Ready(Err(failure));
                }
                Poll::Ready(None) => {
                    if let State::Active(done) = std::mem::replace(&mut self.state, State::Done) {
                        done.lease.finish_clean();
                    }
                    return Poll::Ready(Ok(None));
                }
                Poll::Pending => {
                    let limit = active.idle_limit;
                    let timer = active
                        .idle_timer
                        .get_or_insert_with(|| Box::pin(sleep(limit)));
                    if timer.as_mut().poll(cx).is_ready() {
                        let text = active.redactor.clean(&format!(
                            "no data arrived within the idle timeout of {limit:?}"
                        ));
                        self.state = State::Done;
                        return Poll::Ready(Err(BodyError::IdleTimeout { text }));
                    }
                    return Poll::Pending;
                }
            }
        }
    }
}

fn scrubbed(error: BodyError, redactor: &Redactor) -> BodyError {
    match error {
        BodyError::UnsupportedEncoding { encoding, text } => BodyError::UnsupportedEncoding {
            encoding: redactor.clean(&encoding),
            text: redactor.clean(&text),
        },
        other => other,
    }
}

fn poll_deadline(timer: &mut Option<Pin<Box<tokio::time::Sleep>>>, cx: &mut Context<'_>) -> bool {
    timer
        .as_mut()
        .is_some_and(|sleep| sleep.as_mut().poll(cx).is_ready())
}

fn body_failure(error: &hyper::Error, redactor: &Redactor) -> BodyError {
    let text = redactor.describe(error);
    match classify(error) {
        Cause::Interrupted => BodyError::Interrupted { text },
        Cause::Protocol => BodyError::Protocol { text },
    }
}

impl fmt::Debug for BodyStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let finished = matches!(self.state, State::Done);
        f.debug_struct("BodyStream")
            .field("finished", &finished)
            .finish()
    }
}

impl fmt::Debug for Collected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Collected")
            .field("bytes_len", &self.bytes.len())
            .field("cut", &self.cut)
            .finish()
    }
}
