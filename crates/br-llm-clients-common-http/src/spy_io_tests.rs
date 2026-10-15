use std::io::{self, IoSlice};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::h2state::H2State;
use crate::h2state_tests::bare_state;
use crate::spy_io::SettingsSpy;
use crate::support::frames::settings_frame;

struct Recorder {
    log: Arc<Mutex<Vec<&'static str>>>,
    to_read: Vec<u8>,
}

impl Recorder {
    fn note(&self, what: &'static str) {
        self.log.lock().expect("log").push(what);
    }
}

impl AsyncRead for Recorder {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let bytes = std::mem::take(&mut self.to_read);
        buf.put_slice(&bytes);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for Recorder {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.note("write");
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.note("flush");
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.note("shutdown");
        Poll::Ready(Ok(()))
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.note("write_vectored");
        Poll::Ready(Ok(bufs.iter().map(|buf| buf.len()).sum()))
    }

    fn is_write_vectored(&self) -> bool {
        true
    }
}

type Log = Arc<Mutex<Vec<&'static str>>>;

fn recorded(to_read: Vec<u8>) -> (SettingsSpy<Recorder>, Log, Arc<H2State>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let recorder = Recorder {
        log: Arc::clone(&log),
        to_read,
    };
    let state = bare_state();
    (SettingsSpy::new(recorder, Arc::clone(&state)), log, state)
}

#[tokio::test]
async fn given_writes_flushes_and_shutdowns_then_each_reaches_the_inner_stream() {
    let (mut spy, log, _) = recorded(Vec::new());
    spy.write_all(b"ab").await.expect("write");
    let written = spy
        .write_vectored(&[IoSlice::new(b"c"), IoSlice::new(b"d")])
        .await
        .expect("vectored");
    assert_eq!(written, 2);
    spy.flush().await.expect("flush");
    spy.shutdown().await.expect("shutdown");
    assert!(spy.is_write_vectored());
    assert_eq!(
        *log.lock().expect("log"),
        ["write", "write_vectored", "flush", "shutdown"]
    );
}

#[tokio::test]
async fn given_a_settings_frame_read_through_the_spy_then_the_bytes_pass_and_the_limit_is_known_at_the_next_read()
 {
    let frame = settings_frame(Some(7));
    let (mut spy, _, state) = recorded(frame.clone());
    assert!(state.awaiting_settings());
    let mut read = vec![0u8; frame.len()];
    spy.read_exact(&mut read).await.expect("read");
    assert_eq!(read, frame);
    assert!(
        state.awaiting_settings(),
        "the limit is announced only once the reader asks for more"
    );
    let mut more = [0u8; 1];
    assert_eq!(spy.read(&mut more).await.expect("read"), 0);
    assert!(!state.awaiting_settings());
}
