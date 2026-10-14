use std::time::Duration;

use br_llm_clients_common_http::{Endpoint, SendError};

use crate::support::canned::serve_bytes;
use crate::support::h2_server::{TlsServer, service};
use crate::support::leaks::*;
use crate::support::pki::Pki;
use crate::support::raw::{RawServer, read_request, wait_for_close};
use crate::support::script_body::ScriptBody;
use crate::support::spy::Spy;
use crate::support::{client, client_with, closed_port_endpoint, guarded};

#[tokio::test]
async fn given_every_failure_before_the_headers_then_no_text_holds_a_secret_a_body_a_header_value_or_a_query()
 {
    let mut all = Vec::new();

    let closed = closed_port_endpoint(false);
    let closed = Endpoint::plain_http(&with_query(&format!(
        "http://127.0.0.1:{}/p",
        closed.port().expect("port")
    )))
    .expect("endpoint");
    all.extend(texts_of_send(&send_failure(&client(), &closed).await));

    let invalid = Spy::returning(&[("x-bad", &format!("{CREDENTIAL}\n"))]);
    let error = guarded(client().send(secret_request(&plain(1)), &invalid))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::InvalidAuthorization { .. }));
    all.extend(texts_of_send(&error));

    let reset_head = serve_bytes(
        format!("HTTP/1.1 200 OK\r\nx-leak: {RESPONSE_HEADER}\r\nx-half: {RESPONSE_BODY}")
            .into_bytes(),
    )
    .await;
    let error = send_failure(&client(), &plain(reset_head.addr.port())).await;
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
    all.extend(texts_of_send(&error));

    let garbage = serve_bytes(
        format!("HTTP/1.1 200 OK\r\n{RESPONSE_HEADER} {RESPONSE_BODY}\r\n\r\n{RESPONSE_BODY}")
            .into_bytes(),
    )
    .await;
    let error = send_failure(&client(), &plain(garbage.addr.port())).await;
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
    all.extend(texts_of_send(&error));

    let silent = RawServer::start(|mut stream, _| async move {
        let _ = read_request(&mut stream).await;
        wait_for_close(&mut stream).await;
    })
    .await;
    let timeout_client = client_with(|c| c.headers_timeout = Duration::from_millis(150));
    let error = send_failure(&timeout_client, &plain(silent.addr.port())).await;
    assert!(matches!(error, SendError::HeadersTimeout { .. }));
    all.extend(texts_of_send(&error));

    let total_client = client_with(|c| c.total_timeout = Some(Duration::from_millis(150)));
    let error = send_failure(&total_client, &plain(silent.addr.port())).await;
    assert!(matches!(error, SendError::TotalTimeout { .. }));
    all.extend(texts_of_send(&error));

    let handshake_stall =
        RawServer::start(|mut stream, _| async move { wait_for_close(&mut stream).await }).await;
    let https = Endpoint::https(&with_query(&format!(
        "https://127.0.0.1:{}/p",
        handshake_stall.addr.port()
    )))
    .expect("endpoint");
    let connect_client = client_with(|c| c.connect_timeout = Duration::from_millis(150));
    let error = send_failure(&connect_client, &https).await;
    assert!(matches!(error, SendError::ConnectTimeout { .. }));
    all.extend(texts_of_send(&error));

    let pki = Pki::other_ca();
    let tls_server = TlsServer::start(
        pki.acceptor(&[b"http/1.1"]),
        service(|_| async { Ok(hyper::Response::new(ScriptBody::new([]))) }),
    )
    .await;
    let tls_endpoint = Endpoint::https(&with_query(&format!(
        "https://127.0.0.1:{}/p",
        tls_server.addr.port()
    )))
    .expect("endpoint");
    let error = send_failure(&client(), &tls_endpoint).await;
    assert!(matches!(error, SendError::Tls { .. }));
    all.extend(texts_of_send(&error));

    assert!(all.len() >= 24);
    assert_clean("send failures", &all);
}
