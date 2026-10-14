use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use br_llm_clients_common_http::{
    ApiKeyAuthorizer, AuthorizationRequest, AuthorizedHeader, Authorizer, AuthorizerError,
    BearerAuthorizer, BoxFuture, ConfigError, NoAuthorizer, Secret, SendError,
};
use tokio::io::AsyncWriteExt;

use crate::support::raw::{RawServer, Received, read_request, write_response};
use crate::support::spy::Spy;
use crate::support::{client, client_with, guarded, post};

async fn echo_server() -> (RawServer, Arc<std::sync::Mutex<Vec<Received>>>) {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let server = RawServer::start(move |mut stream, _| {
        let sink = Arc::clone(&sink);
        async move {
            while let Ok(received) = read_request(&mut stream).await {
                sink.lock().expect("lock").push(received);
                if write_response(&mut stream, "200 OK", &[], b"ok")
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    })
    .await;
    (server, seen)
}

#[tokio::test]
async fn given_an_invalid_returned_header_when_sent_then_nothing_is_sent_and_the_text_hides_the_value()
 {
    let cases: [&[(&str, &str)]; 6] = [
        &[("bad name", "SECRET-V")],
        &[("x-ok", "SECRET-V\n")],
        &[("x-ok", "SECRET-V\r\nx-injected: 1")],
        &[("Host", "SECRET-V")],
        &[("accept-encoding", "SECRET-V")],
        &[("x-a", "SECRET-V"), ("X-A", "SECRET-W")],
    ];
    for headers in cases {
        let (server, seen) = echo_server().await;
        let spy = Spy::returning(headers);
        let error = guarded(client().send(post(&server.endpoint("/"), "a"), &spy))
            .await
            .unwrap_err();
        let SendError::InvalidAuthorization { text } = &error else {
            panic!("{error:?}")
        };
        assert!(!text.contains("SECRET"), "{text}");
        assert!(!format!("{error} {error:?}").contains("SECRET"));
        assert_eq!(server.accepted(), 0);
        assert!(seen.lock().expect("lock").is_empty());
        assert_eq!(spy.calls(), 1);
    }
}

#[tokio::test]
async fn given_an_invalid_returned_header_when_sent_then_the_text_names_the_header() {
    let spy = Spy::returning(&[("x-bad", "tok\n")]);
    let (server, _seen) = echo_server().await;
    let error = guarded(client().send(post(&server.endpoint("/"), ""), &spy))
        .await
        .unwrap_err();
    let SendError::InvalidAuthorization { text } = error else {
        panic!("wrong error")
    };
    assert!(text.contains("x-bad"));
}

#[tokio::test]
async fn given_an_authorizer_failure_when_sent_then_each_case_maps_to_the_authorizer_error_and_nothing_is_sent()
 {
    for failure in [
        AuthorizerError::unavailable("no key"),
        AuthorizerError::fetch_failed("token endpoint down"),
    ] {
        let (server, _seen) = echo_server().await;
        let spy = Spy::failing(failure.clone());
        let error = guarded(client().send(post(&server.endpoint("/"), ""), &spy))
            .await
            .unwrap_err();
        assert_eq!(error, SendError::Authorizer(failure));
        assert_eq!(server.accepted(), 0);
    }
}

#[tokio::test]
async fn given_the_ready_made_authorizers_when_sent_then_they_add_exactly_their_header() {
    let (server, seen) = echo_server().await;
    let key = ApiKeyAuthorizer::new("X-Api-Key", Secret::new("sk-key").expect("secret"))
        .expect("authorizer");
    let bearer = BearerAuthorizer::new(&Secret::new("tok-1").expect("secret"));
    guarded(client().send(post(&server.endpoint("/"), ""), &key))
        .await
        .expect("response");
    guarded(client().send(post(&server.endpoint("/"), ""), &bearer))
        .await
        .expect("response");
    guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let seen = seen.lock().expect("lock");
    assert_eq!(seen[0].header("x-api-key"), Some("sk-key"));
    assert_eq!(seen[0].header_count("authorization"), 0);
    assert_eq!(seen[1].header("authorization"), Some("Bearer tok-1"));
    assert_eq!(seen[1].header_count("x-api-key"), 0);
    assert_eq!(seen[2].header_count("authorization"), 0);
    assert_eq!(seen[2].header_count("x-api-key"), 0);
}

#[tokio::test]
async fn given_an_api_key_authorizer_with_a_bad_or_reserved_name_when_built_then_it_is_refused() {
    let secret = || Secret::new("sk").expect("secret");
    assert!(matches!(
        ApiKeyAuthorizer::new("bad name", secret()),
        Err(ConfigError::InvalidHeader { .. })
    ));
    assert!(matches!(
        ApiKeyAuthorizer::new("Host", secret()),
        Err(ConfigError::ReservedHeader { .. })
    ));
}

#[tokio::test]
async fn given_a_plain_http_endpoint_when_sent_then_credentials_are_sent_like_on_any_other() {
    let (server, seen) = echo_server().await;
    let bearer = BearerAuthorizer::new(&Secret::new("tok-plain").expect("secret"));
    guarded(client().send(post(&server.endpoint("/"), ""), &bearer))
        .await
        .expect("response");
    assert_eq!(
        seen.lock().expect("lock")[0].header("authorization"),
        Some("Bearer tok-plain")
    );
}

struct Hanging {
    dropped: Arc<AtomicBool>,
}

struct SetOnDrop(Arc<AtomicBool>);

impl Drop for SetOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl Authorizer for Hanging {
    fn authorize<'a>(
        &'a self,
        _request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        let guard = SetOnDrop(Arc::clone(&self.dropped));
        Box::pin(async move {
            let _guard = guard;
            std::future::pending::<()>().await;
            Ok(Vec::new())
        })
    }
}

#[tokio::test]
async fn given_an_authorizer_that_hangs_when_the_total_timeout_passes_then_the_send_fails_with_total_timeout()
 {
    let dropped = Arc::new(AtomicBool::new(false));
    let hanging = Hanging {
        dropped: Arc::clone(&dropped),
    };
    let (server, _seen) = echo_server().await;
    let client = client_with(|c| c.total_timeout = Some(Duration::from_millis(200)));
    let error = guarded(client.send(post(&server.endpoint("/"), ""), &hanging))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::TotalTimeout { .. }), "{error:?}");
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(server.accepted(), 0);
}

#[tokio::test]
async fn given_a_send_dropped_while_the_authorizer_waits_then_the_authorizer_future_is_dropped() {
    let dropped = Arc::new(AtomicBool::new(false));
    let hanging = Hanging {
        dropped: Arc::clone(&dropped),
    };
    let (server, _seen) = echo_server().await;
    let client = client();
    let outcome = tokio::time::timeout(
        Duration::from_millis(100),
        client.send(post(&server.endpoint("/"), ""), &hanging),
    )
    .await;
    assert!(outcome.is_err());
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(server.accepted(), 0);
}

#[tokio::test]
async fn given_an_authorizer_that_is_slow_when_no_total_timeout_is_set_then_the_send_waits_for_it()
{
    struct Slow;
    impl Authorizer for Slow {
        fn authorize<'a>(
            &'a self,
            _request: &'a AuthorizationRequest<'a>,
        ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
            Box::pin(async {
                tokio::time::sleep(Duration::from_millis(600)).await;
                Ok(Vec::new())
            })
        }
    }
    let (server, _seen) = echo_server().await;
    let client = client_with(|c| {
        c.connect_timeout = Duration::from_millis(100);
        c.headers_timeout = Duration::from_millis(100);
    });
    let response = guarded(client.send(post(&server.endpoint("/"), ""), &Slow))
        .await
        .expect("response");
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn given_the_server_closing_after_the_credential_was_sent_then_no_call_to_the_authorizer_is_repeated()
 {
    let server = RawServer::start(|mut stream, _| async move {
        let _ = read_request(&mut stream).await;
        let _ = stream.shutdown().await;
    })
    .await;
    let spy = Spy::returning(&[("x-api-key", "k")]);
    let error = guarded(client().send(post(&server.endpoint("/"), ""), &spy))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Interrupted { .. }), "{error:?}");
    assert_eq!(spy.calls(), 1);
}
