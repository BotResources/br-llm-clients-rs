use std::sync::Arc;

use br_llm_clients_common_http::{
    AuthorizationRequest, AuthorizedHeader, Authorizer, AuthorizerError, BoxFuture, HttpClient,
    HttpConfig, Secret,
};
use br_llm_clients_core::ClientError;

use crate::call_spec::CallSpec;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{LONG, failure_of, model, run_call, user_request};
use crate::test_server::{TestServer, closed_endpoint, http, raw};

struct Scripted(Result<Vec<(&'static str, &'static str)>, AuthorizerError>);

impl Authorizer for Scripted {
    fn authorize<'a>(
        &'a self,
        _request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        let result = match &self.0 {
            Ok(headers) => Ok(headers
                .iter()
                .map(|(name, value)| {
                    AuthorizedHeader::new(*name, Secret::new(*value).expect("secret"))
                })
                .collect()),
            Err(error) => Err(error.clone()),
        };
        Box::pin(async move { result })
    }
}

fn spec_with(authorizer: Scripted, endpoint: br_llm_clients_common_http::Endpoint) -> CallSpec {
    CallSpec::new(
        Arc::new(ScriptedDialect::default()),
        endpoint,
        Arc::new(authorizer),
        HttpClient::new(HttpConfig::new(LONG, LONG, LONG)).expect("client"),
        model(),
    )
}

#[tokio::test]
async fn given_an_authorizer_that_has_no_credential_when_run_then_it_is_an_authentication_error() {
    let server = TestServer::start(|_| http("200 OK", &[], b"")).await;
    let authorizer = Scripted(Err(AuthorizerError::unavailable("none")));
    let spec = spec_with(authorizer, server.endpoint("/v1/test"));
    assert_eq!(
        failure_of(&spec, &user_request()).await,
        ClientError::Authentication
    );
    assert_eq!(server.count(), 0);
}

#[tokio::test]
async fn given_an_authorizer_that_cannot_fetch_when_run_then_it_is_a_transport_error() {
    let server = TestServer::start(|_| http("200 OK", &[], b"")).await;
    let authorizer = Scripted(Err(AuthorizerError::fetch_failed("down")));
    let spec = spec_with(authorizer, server.endpoint("/v1/test"));
    assert_eq!(
        failure_of(&spec, &user_request()).await,
        ClientError::Transport
    );
    assert_eq!(server.count(), 0);
}

#[tokio::test]
async fn given_an_invalid_authorization_when_run_then_it_is_an_authentication_error() {
    let server = TestServer::start(|_| http("200 OK", &[], b"")).await;
    let spec = spec_with(
        Scripted(Ok(vec![("bad name", "value")])),
        server.endpoint("/v1/test"),
    );
    assert_eq!(
        failure_of(&spec, &user_request()).await,
        ClientError::Authentication
    );
    assert_eq!(server.count(), 0);
}

#[tokio::test]
async fn given_a_valid_authorization_when_run_then_the_header_reaches_the_server() {
    let server = TestServer::start(|_| http("500 Bad", &[], b"")).await;
    let spec = spec_with(
        Scripted(Ok(vec![("authorization", "Bearer abc")])),
        server.endpoint("/v1/test"),
    );
    run_call(&spec, &user_request()).await;
    assert_eq!(
        server.requests()[0].header_values("authorization"),
        ["Bearer abc"]
    );
}

#[tokio::test]
async fn given_a_refused_connection_when_run_then_it_is_a_transport_error_without_a_request_id() {
    let spec = spec_with(Scripted(Ok(Vec::new())), closed_endpoint());
    let (report, result) = run_call(&spec, &user_request()).await.into_parts();
    assert_eq!(result, Err(ClientError::Transport));
    assert_eq!(report.request_id(), None);
}

#[tokio::test]
async fn given_a_server_that_does_not_speak_http_when_run_then_the_reply_is_malformed() {
    let server = TestServer::start(|_| raw(b"this is not http\r\n\r\n".to_vec())).await;
    let spec = spec_with(Scripted(Ok(Vec::new())), server.endpoint("/v1/test"));
    let error = failure_of(&spec, &user_request()).await;
    assert!(
        matches!(error, ClientError::MalformedResponse { .. }),
        "{error:?}"
    );
    assert_eq!(server.count(), 1);
}
