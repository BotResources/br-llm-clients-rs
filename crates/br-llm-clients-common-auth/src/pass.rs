use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) struct Pass {
    passed: bool,
}

pub(crate) fn pass() -> Pass {
    Pass { passed: false }
}

impl Future for Pass {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.passed {
            return Poll::Ready(());
        }
        self.passed = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
