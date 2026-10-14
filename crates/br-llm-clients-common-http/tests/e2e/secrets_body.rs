use std::time::Duration;

use br_llm_clients_common_http::{BodyError, Endpoint, NoAuthorizer, Request, SendError};
use tokio::io::AsyncWriteExt;

use crate::support::canned::{serve_bytes, serve_response};
use crate::support::leaks::*;
use crate::support::raw::{RawServer, read_request, wait_for_close, write_raw};
use crate::support::spy::Spy;
use crate::support::{client, client_with, closed_port_endpoint, guarded};

#[tokio::test]
async fn given_every_failure_after_the_headers_then_no_text_holds_a_secret_a_body_a_header_value_or_a_query()
 {
    let mut all = Vec::new();

    let cut = serve_bytes(format!("HTTP/1.1 200 OK\r\nx-leak: {RESPONSE_HEADER}\r\ncontent-length: 500\r\nconnection: close\r\n\r\n{RESPONSE_BODY}").into_bytes()).await;
    let error = body_failure(&client(), &plain(cut.addr.port())).await;
    assert!(matches!(error, BodyError::Interrupted { .. }), "{error:?}");
    all.extend(texts_of_body(&error));

    let bad_chunk = serve_bytes(format!("HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n3\r\nabc\r\n{RESPONSE_BODY}\r\n").into_bytes()).await;
    let error = body_failure(&client(), &plain(bad_chunk.addr.port())).await;
    assert!(matches!(error, BodyError::Protocol { .. }), "{error:?}");
    all.extend(texts_of_body(&error));

    let stalled = RawServer::start(|mut stream, _| async move {
        let _ = read_request(&mut stream).await;
        let head = format!(
            "HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nx-leak: {RESPONSE_HEADER}\r\n\r\n"
        );
        let _ = write_raw(&mut stream, head.as_bytes()).await;
        let _ = write_raw(
            &mut stream,
            format!("{:x}\r\n{RESPONSE_BODY}\r\n", RESPONSE_BODY.len()).as_bytes(),
        )
        .await;
        wait_for_close(&mut stream).await;
    })
    .await;
    let idle_client = client_with(|c| c.idle_timeout = Duration::from_millis(150));
    let error = body_failure(&idle_client, &plain(stalled.addr.port())).await;
    assert!(matches!(error, BodyError::IdleTimeout { .. }), "{error:?}");
    all.extend(texts_of_body(&error));

    let total_client = client_with(|c| {
        c.total_timeout = Some(Duration::from_millis(150));
        c.idle_timeout = Duration::from_secs(5);
    });
    let error = body_failure(&total_client, &plain(stalled.addr.port())).await;
    assert!(matches!(error, BodyError::TotalTimeout { .. }), "{error:?}");
    all.extend(texts_of_body(&error));

    let encoded = serve_response(
        "200 OK",
        &[("Content-Encoding", RESPONSE_HEADER)],
        RESPONSE_BODY.as_bytes(),
    )
    .await;
    let error = body_failure(&client(), &plain(encoded.addr.port())).await;
    assert!(
        matches!(error, BodyError::UnsupportedEncoding { .. }),
        "{error:?}"
    );
    all.extend(texts_of_body(&error));

    assert!(all.len() >= 15);
    assert_clean("body failures", &all);
}

#[tokio::test]
async fn given_a_credential_that_appears_in_a_lower_layer_text_then_the_second_barrier_removes_it()
{
    let closed = closed_port_endpoint(false);
    let spy = Spy::returning(&[("authorization", "127.0.0.1")]);
    let error = guarded(client().send(Request::new(Endpoint::clone(&closed), ""), &spy))
        .await
        .unwrap_err();
    let SendError::Connect { text } = &error else {
        panic!("{error:?}")
    };
    assert!(!text.contains("127.0.0.1"), "{text}");
    assert!(text.contains("[redacted]"), "{text}");
}

#[tokio::test]
async fn given_a_credential_equal_to_a_timeout_text_then_it_is_removed_from_the_timeout_text() {
    let silent = RawServer::start(|mut stream, _| async move {
        let _ = read_request(&mut stream).await;
        let _ = stream.shutdown().await;
        wait_for_close(&mut stream).await;
    })
    .await;
    let client = client_with(|c| c.headers_timeout = Duration::from_millis(150));
    let spy = Spy::returning(&[("authorization", "150ms")]);
    let silent_forever =
        RawServer::start(|mut stream, _| async move { wait_for_close(&mut stream).await }).await;
    let error = guarded(client.send(Request::new(plain(silent_forever.addr.port()), ""), &spy))
        .await
        .unwrap_err();
    let SendError::HeadersTimeout { text } = &error else {
        panic!("{error:?}")
    };
    assert!(!text.contains("150ms"), "{text}");
    drop(silent);
}

#[tokio::test]
async fn given_the_values_of_a_successful_call_then_nothing_secret_is_printed_by_response_or_stream_debug()
 {
    let server = serve_response(
        "200 OK",
        &[("X-Leak", RESPONSE_HEADER)],
        RESPONSE_BODY.as_bytes(),
    )
    .await;
    let mut response =
        guarded(client().send(secret_request(&plain(server.addr.port())), &NoAuthorizer))
            .await
            .expect("response");
    let mut texts = vec![
        format!("{response:?}"),
        format!("{:#?}", response.headers()),
        format!("{:?}", response.body_mut()),
    ];
    let collected = response.body_mut().collect(100).await.expect("body");
    texts.push(format!("{collected:?} {collected:#?}"));
    assert_clean("response", &texts);
}
