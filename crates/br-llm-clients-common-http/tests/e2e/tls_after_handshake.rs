use br_llm_clients_common_http::{BodyError, Endpoint, NoAuthorizer, Request, SendError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::support::frames;
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

async fn corrupting_h2_server(alert_instead: bool, after_headers: bool) -> RawServer {
    let acceptor = Pki::trusted().acceptor(&[b"h2"]);
    RawServer::start(move |tcp, _| {
        let acceptor = acceptor.clone();
        async move {
            let Ok(mut tls) = acceptor.accept(tcp).await else {
                return;
            };
            if !frames::read_preface(&mut tls).await {
                return;
            }
            frames::write_all(&mut tls, &frames::settings_frame(None)).await;
            let mut request_stream = 0;
            while let Some(received) = frames::read_frame(&mut tls).await {
                if received.kind == frames::HEADERS {
                    request_stream = received.stream;
                    break;
                }
            }
            if after_headers {
                let head = frames::frame(
                    frames::HEADERS,
                    frames::END_HEADERS,
                    request_stream,
                    &[frames::STATUS_200],
                );
                frames::write_all(&mut tls, &head).await;
                let data = frames::frame(frames::DATA, 0, request_stream, b"abc");
                frames::write_all(&mut tls, &data).await;
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            let (mut tcp, _) = tls.into_inner();
            let record: &[u8] = if alert_instead {
                &[0x15, 0x03, 0x03, 0x00, 0x02, 0x02, 0x28]
            } else {
                &[0x17, 0x03, 0x03, 0x00, 0x05, 1, 2, 3, 4, 5]
            };
            let _ = tcp.write_all(record).await;
        }
    })
    .await
}

#[tokio::test]
async fn given_a_corrupt_or_unexpected_tls_record_over_http2_before_the_headers_then_the_send_is_interrupted_not_protocol()
 {
    for alert in [false, true] {
        let server = corrupting_h2_server(alert, false).await;
        let error = guarded(
            trusting_client(&Pki::trusted().ca_pem)
                .send(Request::new(https(&server), "b"), &NoAuthorizer),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error, SendError::Interrupted { .. }),
            "alert={alert}: {error:?}"
        );
    }
}

#[tokio::test]
async fn given_a_corrupt_or_unexpected_tls_record_over_http2_in_the_body_then_the_stream_is_interrupted_not_protocol()
 {
    for alert in [false, true] {
        let server = corrupting_h2_server(alert, true).await;
        let mut response = guarded(
            trusting_client(&Pki::trusted().ca_pem)
                .send(Request::new(https(&server), "b"), &NoAuthorizer),
        )
        .await
        .expect("the headers arrived");
        let error = response.body_mut().collect(100).await.unwrap_err();
        assert!(
            matches!(error, BodyError::Interrupted { .. }),
            "alert={alert}: {error:?}"
        );
    }
}
