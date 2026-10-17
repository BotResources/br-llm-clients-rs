use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use hyper::body::{Body, Frame};

pub enum Step {
    Data(&'static [u8]),
    Fail,
    Stall,
    Pause(std::time::Duration),
}

pub struct ScriptBody {
    steps: VecDeque<Step>,
    pause: Option<Pin<Box<tokio::time::Sleep>>>,
}

impl ScriptBody {
    pub fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            pause: None,
        }
    }
}

impl Body for ScriptBody {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if matches!(self.steps.front(), Some(Step::Stall)) {
            return Poll::Pending;
        }
        if let Some(Step::Pause(span)) = self.steps.front() {
            let span = *span;
            let sleep = self
                .pause
                .get_or_insert_with(|| Box::pin(tokio::time::sleep(span)));
            if sleep.as_mut().poll(cx).is_pending() {
                return Poll::Pending;
            }
            self.pause = None;
            self.steps.pop_front();
        }
        match self.steps.pop_front() {
            None | Some(Step::Stall | Step::Pause(_)) => Poll::Ready(None),
            Some(Step::Data(bytes)) => {
                Poll::Ready(Some(Ok(Frame::data(Bytes::from_static(bytes)))))
            }
            Some(Step::Fail) => Poll::Ready(Some(Err(io::Error::other("scripted failure")))),
        }
    }
}
