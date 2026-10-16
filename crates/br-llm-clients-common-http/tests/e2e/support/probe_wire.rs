use std::time::Duration;

use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::frames::{ACK, HEADERS, SETTINGS, frame, goaway_frame, read_frame, settings_frame};
use super::probe_types::{Queued, Records};

pub(super) async fn read_body_driving<T>(
    conn: &mut h2::server::Connection<T, Bytes>,
    queued: &mut Queued,
    mut body: h2::RecvStream,
) -> Vec<u8>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut flow = body.flow_control().clone();
    let mut bytes = Vec::new();
    let mut accepting = true;
    loop {
        tokio::select! {
            chunk = body.data() => match chunk {
                Some(Ok(chunk)) => {
                    let _ = flow.release_capacity(chunk.len());
                    bytes.extend_from_slice(&chunk);
                }
                _ => return bytes,
            },
            accepted = conn.accept(), if accepting => match accepted {
                Some(Ok(next)) => queued.push_back(next),
                _ => accepting = false,
            },
        }
    }
}

const QUIET: Duration = Duration::from_millis(20);

pub(super) async fn gather_more<T>(conn: &mut h2::server::Connection<T, Bytes>, records: &Records)
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut held = Vec::new();
    while let Ok(Some(Ok((request, respond)))) = tokio::time::timeout(QUIET, conn.accept()).await {
        let id = request
            .headers()
            .get("x-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        records.note_arrival(id);
        held.push((request, respond));
    }
}

pub async fn go_away_with<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    mut stream: S,
    last_stream: u32,
    code: u32,
    debug: &[u8],
) -> bool {
    let mut preface = [0u8; 24];
    if stream.read_exact(&mut preface).await.is_err() {
        return false;
    }
    let _ = stream.write_all(&settings_frame(None)).await;
    while let Some(received) = read_frame(&mut stream).await {
        if received.kind == SETTINGS && received.flags & ACK == 0 {
            let _ = stream.write_all(&frame(SETTINGS, ACK, 0, &[])).await;
        }
        if received.kind == HEADERS {
            let _ = stream
                .write_all(&goaway_frame(last_stream, code, debug))
                .await;
            let _ = stream.flush().await;
            if last_stream > 0 {
                return true;
            }
            let _ = stream.shutdown().await;
            let mut sink = [0u8; 1024];
            let lingering =
                async { while matches!(stream.read(&mut sink).await, Ok(n) if n > 0) {} };
            let _ = tokio::time::timeout(Duration::from_millis(500), lingering).await;
            return true;
        }
    }
    false
}
