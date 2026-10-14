use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};

pub(crate) struct Payload(Option<Bytes>);

impl Payload {
    pub(crate) fn new(bytes: Bytes) -> Self {
        Self((!bytes.is_empty()).then_some(bytes))
    }
}

impl Body for Payload {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(self.0.take().map(|bytes| Ok(Frame::data(bytes))))
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_none()
    }

    fn size_hint(&self) -> SizeHint {
        let len = self.0.as_ref().map_or(0, Bytes::len);
        SizeHint::with_exact(u64::try_from(len).unwrap_or(u64::MAX))
    }
}
