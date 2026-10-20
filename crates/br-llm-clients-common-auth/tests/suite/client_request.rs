use std::time::Duration;

use br_llm_clients_common_auth::{FormTokenClient, TokenForm};
use br_llm_clients_common_http::{AuthorizerError, Secret};

use crate::support::server::{Server, http_client, json, response};

const OK: &str = r#"{"access_token":"at"}"#;
const MANAGED: [&str; 5] = [
    "content-type",
    "host",
    "content-length",
    "user-agent",
    "accept-encoding",
];

fn client(server: &Server, path_and_query: &str) -> FormTokenClient {
    FormTokenClient::new(
        http_client(Duration::from_secs(5)),
        server.endpoint(path_and_query),
    )
}

fn secret(value: &str) -> Secret {
    Secret::new(value).expect("secret")
}

#[tokio::test]
async fn given_a_form_when_fetched_then_one_post_goes_to_the_endpoint_with_its_path_and_query() {
    let server = Server::start(vec![json(200, OK)]).await;
    let form = TokenForm::refresh_token(secret("rt"));
    client(&server, "/tenant/oauth2/token?api-version=2")
        .fetch(&form)
        .await
        .expect("reply");
    let captured = server.captured();
    assert_eq!(captured.len(), 1);
    assert_eq!(
        captured[0].request_line,
        "POST /tenant/oauth2/token?api-version=2 HTTP/1.1"
    );
}

#[tokio::test]
async fn given_a_form_when_fetched_then_the_library_sets_only_the_form_content_type() {
    let server = Server::start(vec![json(200, OK)]).await;
    let form = TokenForm::refresh_token(secret("rt"));
    client(&server, "/token").fetch(&form).await.expect("reply");
    let captured = server.captured();
    assert_eq!(
        captured[0].header_values("content-type"),
        vec!["application/x-www-form-urlencoded"]
    );
    assert!(captured[0].header_values("authorization").is_empty());
    for (name, _) in &captured[0].headers {
        assert!(
            MANAGED.contains(&name.to_ascii_lowercase().as_str()),
            "unexpected header {name}"
        );
    }
}

#[tokio::test]
async fn given_fields_when_fetched_then_the_body_holds_them_in_order_and_encoded() {
    let server = Server::start(vec![json(200, OK)]).await;
    let form = TokenForm::client_credentials("my id", secret("s&=+\u{e9}*"))
        .expect("form")
        .plain("scope", "api://x/.default")
        .expect("scope")
        .plain("a b", "~-._")
        .expect("name");
    client(&server, "/token").fetch(&form).await.expect("reply");
    let body = String::from_utf8(server.captured()[0].body.clone()).expect("utf8");
    assert_eq!(
        body,
        "grant_type=client_credentials&client_id=my%20id&client_secret=s%26%3D%2B%C3%A9%2A\
         &scope=api%3A%2F%2Fx%2F.default&a%20b=~-._"
    );
}

#[tokio::test]
async fn given_a_redirect_then_it_is_not_followed_and_it_is_unavailable() {
    let server = Server::start(vec![response(302, &[("location", "/elsewhere")], b"")]).await;
    let form = TokenForm::refresh_token(secret("rt"));
    let error = client(&server, "/token")
        .fetch(&form)
        .await
        .expect_err("redirect");
    assert_eq!(
        error,
        AuthorizerError::unavailable("the token endpoint answered 302")
    );
    assert_eq!(server.captured().len(), 1);
}

#[tokio::test]
async fn given_a_server_error_then_the_request_is_not_retried() {
    let server = Server::start(vec![json(503, "{}"), json(200, OK)]).await;
    let form = TokenForm::refresh_token(secret("rt"));
    let error = client(&server, "/token")
        .fetch(&form)
        .await
        .expect_err("server error");
    assert_eq!(
        error,
        AuthorizerError::fetch_failed("the token endpoint answered 503")
    );
    assert_eq!(server.captured().len(), 1);
}
