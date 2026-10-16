use bytes::Bytes;
use h2::Reason;
use tokio::io::{AsyncRead, AsyncWrite};

use super::probe::{Arrived, Queued, Records, Script};
use super::probe_wire::{gather_more, go_away_with, read_body_driving};

fn refuses(script: Script, id: &str, records: &Records) -> bool {
    match script {
        Script::RefuseAll => true,
        Script::RefuseFirst(n) => records.count_refusal() < n,
        Script::RefuseEvery(n) => {
            records.is_first_copy(id)
                && id
                    .trim_start_matches('r')
                    .parse::<usize>()
                    .is_ok_and(|number| number % n == 0)
        }
        _ => false,
    }
}

pub(super) async fn serve<S>(script: Script, index: usize, tls: S, records: Records)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    if let Script::GoAway {
        first_connections,
        last_stream,
        code,
    } = script
        && index < first_connections
    {
        if go_away_with(tls, last_stream, code, b"").await {
            records.note_arrival("?".to_owned());
        }
        return;
    }
    let mut builder = h2::server::Builder::new();
    builder.max_concurrent_streams(1000);
    if matches!(script, Script::ZeroWindowClose | Script::ZeroWindowGoAway) {
        builder.initial_window_size(0);
    }
    let Ok(mut conn) = builder.handshake::<_, Bytes>(tls).await else {
        return;
    };
    let mut served = 0usize;
    let mut queued = Queued::new();
    loop {
        let next = match queued.pop_front() {
            Some(next) => next,
            None => match conn.accept().await {
                Some(Ok(next)) => next,
                _ => break,
            },
        };
        let (request, mut respond) = next;
        served = served.saturating_add(1);
        if matches!(script, Script::Graceful(after) if after == served) {
            conn.graceful_shutdown();
        }
        let id = request
            .headers()
            .get("x-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        records.note_arrival(id.clone());
        let (parts, body) = request.into_parts();
        let key = parts
            .headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let body = if matches!(script, Script::RefuseFirst(_)) {
            Some(read_body_driving(&mut conn, &mut queued, body).await)
        } else {
            None
        };
        records.note_arrived(Arrived {
            id: id.clone(),
            key,
            body,
        });
        if refuses(script, &id, &records) {
            respond.send_reset(Reason::REFUSED_STREAM);
            continue;
        }
        if let Script::ResetAll(reason) = script {
            respond.send_reset(reason);
            continue;
        }
        if matches!(
            script,
            Script::CloseAfterReceiving | Script::ZeroWindowClose
        ) {
            gather_more(&mut conn, &records).await;
            return;
        }
        if matches!(
            script,
            Script::GoAwayAfterReceiving | Script::ZeroWindowGoAway
        ) {
            gather_more(&mut conn, &records).await;
            conn.abrupt_shutdown(Reason::NO_ERROR);
            while conn.accept().await.is_some() {}
            return;
        }
        records.note_processed(id);
        let ok = http::Response::builder()
            .status(200)
            .body(())
            .expect("response");
        if let Ok(mut stream) = respond.send_response(ok, false) {
            let _ = stream.send_data(Bytes::from_static(b"ok"), true);
        }
    }
}
