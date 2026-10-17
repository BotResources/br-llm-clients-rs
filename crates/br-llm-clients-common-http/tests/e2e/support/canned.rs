use tokio::io::AsyncWriteExt;

use super::raw::{RawServer, read_request, write_raw};

pub async fn serve_bytes(wire: Vec<u8>) -> RawServer {
    RawServer::start(move |mut stream, _| {
        let wire = wire.clone();
        async move {
            if read_request(&mut stream).await.is_err() {
                return;
            }
            let _ = write_raw(&mut stream, &wire).await;
            let _ = stream.shutdown().await;
        }
    })
    .await
}

pub async fn serve_response(status: &str, headers: &[(&str, &str)], body: &[u8]) -> RawServer {
    let mut wire = format!("HTTP/1.1 {status}\r\n").into_bytes();
    for (name, value) in headers {
        wire.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    wire.extend_from_slice(
        format!(
            "content-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        )
        .as_bytes(),
    );
    wire.extend_from_slice(body);
    serve_bytes(wire).await
}
