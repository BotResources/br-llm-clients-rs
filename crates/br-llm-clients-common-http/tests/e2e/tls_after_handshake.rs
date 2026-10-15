use br_llm_clients_common_http::{BodyError, Endpoint, NoAuthorizer, Request, SendError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::support::pki::Pki;
use crate::support::raw::RawServer;
use crate::support::{guarded, trusting_client};

fn https(server: &RawServer) -> Endpoint {
    Endpoint::https(&format!("https://127.0.0.1:{}/x", server.addr.port())).expect("endpoint")
}

async fn serve(close_notify: bool, corrupt: bool) -> RawServer {
    let acceptor = Pki::trusted().acceptor(&[b"http/1.1"]);
    RawServer::start(move |tcp, _| {
        let acceptor = acceptor.clone();
        async move {
            let Ok(mut tls) = acceptor.accept(tcp).await else {
                return;
            };
            let mut buffer = vec![0u8; 8192];
            let _ = tls.read(&mut buffer).await;
            if corrupt {
                let (mut tcp, _) = tls.into_inner();
                let _ = tcp
                    .write_all(&[0x17, 0x03, 0x03, 0x00, 0x05, 1, 2, 3, 4, 5])
                    .await;
                return;
            }
            let _ = tls
                .write_all(b"HTTP/1.1 200 OK\r\nconnection: close\r\n\r\nhello world")
                .await;
            let _ = tls.flush().await;
            if close_notify {
                let _ = tls.shutdown().await;
            }
        }
    })
    .await
}

#[tokio::test]
async fn given_a_corrupt_tls_record_after_the_request_was_written_then_the_send_is_interrupted() {
    let server = serve(false, true).await;
    let error = guarded(
        trusting_client(&Pki::trusted().ca_pem)
            .send(Request::new(https(&server), "b"), &NoAuthorizer),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_body_read_until_close_over_tls_then_it_ends_cleanly_only_with_a_close_notify() {
    let with = serve(true, false).await;
    let mut response = guarded(
        trusting_client(&Pki::trusted().ca_pem)
            .send(Request::new(https(&with), "b"), &NoAuthorizer),
    )
    .await
    .expect("response");
    let collected = response.body_mut().collect(100).await.expect("clean end");
    assert_eq!(collected.bytes, b"hello world");

    let without = serve(false, false).await;
    let mut response = guarded(
        trusting_client(&Pki::trusted().ca_pem)
            .send(Request::new(https(&without), "b"), &NoAuthorizer),
    )
    .await
    .expect("response");
    let error = response.body_mut().collect(100).await.unwrap_err();
    assert!(matches!(error, BodyError::Interrupted { .. }), "{error:?}");
}
