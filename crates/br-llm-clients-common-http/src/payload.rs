use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};

pub(crate) struct Payload {
    data: Option<Bytes>,
    taken: Arc<AtomicBool>,
}

impl Payload {
    pub(crate) fn new(bytes: Bytes, taken: Arc<AtomicBool>) -> Self {
        Self {
            data: (!bytes.is_empty()).then_some(bytes),
            taken,
        }
    }

    fn mark(&self) {
        self.taken.store(true, Ordering::SeqCst);
    }
}

impl Body for Payload {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.mark();
        Poll::Ready(self.data.take().map(|bytes| Ok(Frame::data(bytes))))
    }

    fn is_end_stream(&self) -> bool {
        self.mark();
        self.data.is_none()
    }

    fn size_hint(&self) -> SizeHint {
        self.mark();
        let len = self.data.as_ref().map_or(0, Bytes::len);
        SizeHint::with_exact(u64::try_from(len).unwrap_or(u64::MAX))
    }
}
