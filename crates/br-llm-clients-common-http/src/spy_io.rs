use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::h2state::H2State;
use crate::settings_watch::SettingsWatch;

pub(crate) struct SettingsSpy<T> {
    inner: T,
    watch: SettingsWatch,
    state: Arc<H2State>,
    announced: Vec<Option<u32>>,
}

impl<T> SettingsSpy<T> {
    pub(crate) fn new(inner: T, state: Arc<H2State>) -> Self {
        Self {
            inner,
            watch: SettingsWatch::default(),
            state,
            announced: Vec::new(),
        }
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for SettingsSpy<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = &mut *self;
        if !this.announced.is_empty() {
            this.state.apply_settings_run(&this.announced);
            this.announced.clear();
        }
        let before = buf.filled().len();
        let polled = Pin::new(&mut self.inner).poll_read(cx, buf);
        if matches!(polled, Poll::Ready(Ok(()))) {
            let this = &mut *self;
            let fresh = buf.filled().get(before..).unwrap_or_default();
            let announced = &mut this.announced;
            this.watch.feed(fresh, &mut |max| announced.push(max));
        }
        polled
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for SettingsSpy<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, bytes)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}
