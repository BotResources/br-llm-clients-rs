use br_llm_clients_common_http::{BodyError, NoAuthorizer};

use crate::support::canned::serve_response;
use crate::support::{client, guarded, post};

async fn first_read(encoding: &str, body: &[u8]) -> (u16, Result<Option<bytes::Bytes>, BodyError>) {
    let server = serve_response("200 OK", &[("Content-Encoding", encoding)], body).await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let status = response.status();
    (status, guarded(response.body_mut().next_chunk()).await)
}

#[tokio::test]
async fn given_another_encoding_when_first_read_then_it_fails_without_naming_it_whatever_the_body()
{
    for (encoding, body) in [
        ("gzip", b"\x1f\x8b\x08".as_slice()),
        ("GZIP", b"x"),
        ("x-brotli", b""),
        ("Zstd", b"abc"),
        ("sk-odd-coding", b""),
    ] {
        let (status, first) = first_read(encoding, body).await;
        assert_eq!(status, 200);
        match first {
            Err(error @ BodyError::UnsupportedEncoding { .. }) => {
                let shown = format!("{error} {error:?}");
                assert!(
                    !shown.to_lowercase().contains(&encoding.to_lowercase()),
                    "{shown}"
                );
            }
            other => panic!("{encoding}: {other:?}"),
        }
    }
}

#[tokio::test]
async fn given_an_unsupported_encoding_when_the_response_is_kept_then_the_importer_reads_the_header()
 {
    let server = serve_response("200 OK", &[("Content-Encoding", "gzip")], b"zz").await;
    let response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        response.headers().get_text("content-encoding"),
        Some("gzip")
    );
}

#[tokio::test]
async fn given_identity_in_any_case_when_read_then_the_body_comes_through() {
    for encoding in ["identity", "Identity", "IDENTITY"] {
        let (_, first) = first_read(encoding, b"plain").await;
        assert_eq!(
            first.expect("chunk").as_deref(),
            Some(b"plain".as_slice()),
            "{encoding}"
        );
    }
}

#[tokio::test]
async fn given_an_unsupported_encoding_when_read_after_the_failure_then_the_stream_is_finished() {
    let server = serve_response("200 OK", &[("content-encoding", "gzip")], b"zz").await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    assert!(response.body_mut().next_chunk().await.is_err());
    assert_eq!(
        response.body_mut().next_chunk().await.expect("finished"),
        None
    );
}

#[tokio::test]
async fn given_an_unsupported_encoding_when_collecting_then_the_failure_is_returned() {
    let server = serve_response(
        "500 Internal Server Error",
        &[("content-encoding", "gzip")],
        b"",
    )
    .await;
    let mut response = guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(response.status(), 500);
    let error = response.body_mut().collect(10).await.unwrap_err();
    assert!(matches!(error, BodyError::UnsupportedEncoding { .. }));
}
