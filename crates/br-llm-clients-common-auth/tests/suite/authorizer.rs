use std::sync::{Arc, Mutex};
use std::time::Duration;

use br_llm_clients_common_auth::{StaticToken, TokenAuthorizer, TokenCache};
use br_llm_clients_common_http::{AuthorizerError, NoAuthorizer, Request, Secret, SendError};

use crate::support::cache_of;
use crate::support::server::{Server, http_client, json};
use crate::support::source::{Scripted, Step};

fn request(server: &Server) -> Request {
    Request::new(server.endpoint("/v1/messages"), b"{}".to_vec())
}

type Seen = Arc<Mutex<Vec<String>>>;
type Observer = Arc<dyn Fn(&Secret) + Send + Sync>;

fn recorder() -> (Seen, Observer) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let observer: Observer = Arc::new(move |token: &Secret| {
        log.lock().expect("log").push(token.expose().to_owned());
    });
    (seen, observer)
}

#[tokio::test]
async fn given_a_token_when_a_request_is_sent_then_one_authorization_header_carries_the_bearer_token()
 {
    let server = Server::start(vec![json(200, "{}")]).await;
    let source = Scripted::new(vec![Step::secs("tok-abc", 1000)]);
    let authorizer = TokenAuthorizer::bearer(cache_of(&source));
    let client = http_client(Duration::from_secs(5));
    client
        .send(request(&server), &authorizer)
        .await
        .expect("sent");
    client
        .send(request(&server), &NoAuthorizer)
        .await
        .expect("sent");
    let captured = server.captured();
    assert_eq!(
        captured[0].header_values("authorization"),
        vec!["Bearer tok-abc"]
    );
    let added: Vec<&str> = captured[0]
        .headers
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| !captured[1].headers.iter().any(|(other, _)| other == name))
        .collect();
    assert_eq!(added, vec!["authorization"]);
}

#[tokio::test]
async fn given_a_cache_that_fails_when_a_request_is_sent_then_the_error_is_unchanged_and_nothing_is_sent()
 {
    let server = Server::start(vec![json(200, "{}")]).await;
    let error = AuthorizerError::unavailable("no credential");
    let source = Scripted::new(vec![Step::fail(error.clone())]);
    let (seen, observer) = recorder();
    let authorizer = TokenAuthorizer::bearer(cache_of(&source)).with_token_observer(observer);
    let failure = http_client(Duration::from_secs(5))
        .send(request(&server), &authorizer)
        .await
        .expect_err("refused");
    assert_eq!(failure, SendError::Authorizer(error));
    assert!(server.captured().is_empty());
    assert!(seen.lock().expect("log").is_empty());
}

#[tokio::test]
async fn given_an_observer_when_a_token_is_given_then_it_is_called_once_for_each_call() {
    let server = Server::start(vec![json(200, "{}")]).await;
    let source = Scripted::new(vec![Step::secs("tok-abc", 1000)]);
    let (seen, observer) = recorder();
    let authorizer = TokenAuthorizer::bearer(cache_of(&source)).with_token_observer(observer);
    let client = http_client(Duration::from_secs(5));
    client
        .send(request(&server), &authorizer)
        .await
        .expect("first");
    assert_eq!(*seen.lock().expect("log"), vec!["tok-abc"]);
    client
        .send(request(&server), &authorizer)
        .await
        .expect("second");
    assert_eq!(*seen.lock().expect("log"), vec!["tok-abc", "tok-abc"]);
    assert_eq!(source.calls(), 1);
}

#[tokio::test]
async fn given_a_token_that_is_not_a_header_value_then_the_observer_is_called_and_the_http_client_refuses()
 {
    let server = Server::start(vec![json(200, "{}")]).await;
    let source = Scripted::new(vec![Step::secs("line\nbreak", 1000)]);
    let (seen, observer) = recorder();
    let authorizer = TokenAuthorizer::bearer(cache_of(&source)).with_token_observer(observer);
    let failure = http_client(Duration::from_secs(5))
        .send(request(&server), &authorizer)
        .await
        .expect_err("refused");
    assert!(matches!(failure, SendError::InvalidAuthorization { .. }));
    assert_eq!(*seen.lock().expect("log"), vec!["line\nbreak"]);
    assert!(server.captured().is_empty());
}

#[tokio::test]
async fn given_a_source_with_no_validity_when_requests_are_sent_then_the_cache_asks_it_for_each_one()
 {
    let server = Server::start(vec![json(200, "{}")]).await;
    let token = Secret::new("static-key").expect("secret");
    let source = Scripted::new(vec![Step::token("static-key", None)]);
    let authorizer = TokenAuthorizer::bearer(cache_of(&source));
    let client = http_client(Duration::from_secs(5));
    client
        .send(request(&server), &authorizer)
        .await
        .expect("first");
    client
        .send(request(&server), &authorizer)
        .await
        .expect("second");
    assert_eq!(source.calls(), 2);
    let plain = TokenAuthorizer::bearer(TokenCache::new(Arc::new(StaticToken::new(token))));
    client.send(request(&server), &plain).await.expect("third");
    let captured = server.captured();
    assert_eq!(captured.len(), 3);
    assert!(
        captured
            .iter()
            .all(|c| c.header_values("authorization") == vec!["Bearer static-key"])
    );
}
