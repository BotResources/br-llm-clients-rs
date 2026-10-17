use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use br_llm_clients_common_http::{BodyError, NoAuthorizer, SendError};
use hyper::{Response, Version};

use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{guarded, post, trusting_client};

async fn h2_server(steps: impl Fn() -> Vec<Step> + Send + Sync + 'static) -> TlsServer {
    TlsServer::start(
        Pki::trusted().acceptor(&[b"h2"]),
        service(move |req| {
            assert_eq!(req.version, Version::HTTP_2);
            let steps = steps();
            async move { Ok(Response::new(ScriptBody::new(steps))) }
        }),
    )
    .await
}

#[tokio::test]
async fn given_concurrent_requests_when_a_connection_is_open_then_they_share_one_http2_connection()
{
    let server = h2_server(|| vec![Step::Data(b"ok")]).await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    guarded(client.send(post(&server.endpoint("/"), "warm"), &NoAuthorizer))
        .await
        .expect("warm-up");
    let mut tasks = Vec::new();
    for i in 0..16 {
        let client = client.clone();
        let endpoint = server.endpoint("/");
        tasks.push(tokio::spawn(async move {
            let mut response = client
                .send(post(&endpoint, &format!("r{i}")), &NoAuthorizer)
                .await
                .expect("response");
            response.body_mut().collect(10).await.expect("body").bytes
        }));
    }
    for task in tasks {
        assert_eq!(guarded(task).await.expect("task"), b"ok");
    }
    assert_eq!(server.accepted(), 1);
}

#[tokio::test]
async fn given_http2_when_headers_repeat_then_names_are_lower_case_and_status_is_a_number() {
    let server = TlsServer::start(
        Pki::trusted().acceptor(&[b"h2"]),
        service(|_| async {
            let mut response = Response::new(ScriptBody::new([Step::Data(b"x")]));
            *response.status_mut() = hyper::StatusCode::TOO_MANY_REQUESTS;
            response
                .headers_mut()
                .append("X-Multi", "a".parse().expect("value"));
            response
                .headers_mut()
                .append("x-multi", "b".parse().expect("value"));
            response
                .headers_mut()
                .append("retry-after", "4".parse().expect("value"));
            Ok(response)
        }),
    )
    .await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(response.status(), 429);
    let values: Vec<&[u8]> = response.headers().get_all("X-MULTI").collect();
    assert_eq!(values, [b"a".as_slice(), b"b"]);
    assert_eq!(response.retry_after(), Some(Duration::from_secs(4)));
}

#[tokio::test]
async fn given_a_stream_reset_in_the_middle_of_the_body_then_it_is_interrupted() {
    let server = h2_server(|| {
        vec![
            Step::Data(b"part"),
            Step::Pause(Duration::from_millis(100)),
            Step::Fail,
        ]
    })
    .await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let mut response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    let error = loop {
        match guarded(response.body_mut().next_chunk()).await {
            Ok(Some(_)) => {}
            Ok(None) => panic!("the reset was not detected"),
            Err(error) => break error,
        }
    };
    assert!(matches!(error, BodyError::Interrupted { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_stream_reset_before_the_headers_then_the_send_is_interrupted() {
    let server = TlsServer::start(
        Pki::trusted().acceptor(&[b"h2"]),
        service(|_| async { Err(std::io::Error::other("refuse")) }),
    )
    .await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let error = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_dropped_stream_then_the_shared_http2_connection_stays_usable() {
    let count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&count);
    let server = TlsServer::start(
        Pki::trusted().acceptor(&[b"h2"]),
        service(move |_| {
            let first = counter.fetch_add(1, Ordering::SeqCst) == 0;
            async move {
                let steps = if first {
                    vec![Step::Data(b"a"), Step::Stall]
                } else {
                    vec![Step::Data(b"b")]
                };
                Ok(Response::new(ScriptBody::new(steps)))
            }
        }),
    )
    .await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let mut first = guarded(client.send(post(&server.endpoint("/"), "1"), &NoAuthorizer))
        .await
        .expect("first");
    first.body_mut().next_chunk().await.expect("chunk");
    drop(first);
    let mut second = guarded(client.send(post(&server.endpoint("/"), "2"), &NoAuthorizer))
        .await
        .expect("second");
    assert_eq!(
        second.body_mut().collect(10).await.expect("body").bytes,
        b"b"
    );
    assert_eq!(server.accepted(), 1);
}

async fn padded_headers_server(count: usize) -> TlsServer {
    TlsServer::start(
        Pki::trusted().acceptor(&[b"h2"]),
        service(move |_| async move {
            let mut response = Response::new(ScriptBody::new([]));
            for i in 0..count {
                let name = hyper::header::HeaderName::from_bytes(format!("x-pad-{i}").as_bytes())
                    .expect("name");
                response
                    .headers_mut()
                    .append(name, "p".repeat(4000).parse().expect("value"));
            }
            Ok(response)
        }),
    )
    .await
}

#[tokio::test]
async fn given_an_oversized_http2_header_block_then_it_is_a_protocol_error() {
    let client = trusting_client(&Pki::trusted().ca_pem);
    for count in [17, 40] {
        let server = padded_headers_server(count).await;
        let error = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
            .await
            .unwrap_err();
        assert!(
            matches!(error, SendError::Protocol { .. }),
            "{count}: {error:?}"
        );
    }
}

#[tokio::test]
async fn given_an_http2_header_block_under_the_limit_then_it_is_accepted() {
    let client = trusting_client(&Pki::trusted().ca_pem);
    let server = padded_headers_server(12).await;
    let response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        response
            .headers()
            .iter()
            .map(|(name, _)| name)
            .filter(|n| n.starts_with("x-pad"))
            .count(),
        12
    );
}

#[tokio::test]
async fn given_an_authority_when_sent_over_http2_then_the_server_sees_the_endpoint_authority_once()
{
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let server = TlsServer::start(
        Pki::trusted().acceptor(&[b"h2"]),
        service(move |req| {
            sink.lock().expect("lock").push((
                req.uri.authority().map(ToString::to_string),
                req.headers.get_all("host").iter().count(),
                req.uri.path_and_query().map(ToString::to_string),
            ));
            async { Ok(Response::new(ScriptBody::new([]))) }
        }),
    )
    .await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let endpoint = server.endpoint("/v1/a%2Fb?x=1");
    guarded(client.send(post(&endpoint, "a"), &NoAuthorizer))
        .await
        .expect("response");
    let seen = seen.lock().expect("lock");
    assert_eq!(
        seen[0].0.as_deref(),
        Some(format!("127.0.0.1:{}", server.addr.port()).as_str())
    );
    assert_eq!(seen[0].1, 0);
    assert_eq!(seen[0].2.as_deref(), Some("/v1/a%2Fb?x=1"));
}
