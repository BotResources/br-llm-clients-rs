use std::time::Duration;

use br_llm_clients_common_auth::{FormTokenClient, TokenForm};
use br_llm_clients_common_http::{AuthorizerError, Secret};

use crate::support::server::{Server, http_client, json, short_body};

const SECRET: &str = "FORM-SECRET-VALUE";

async fn refused(status: u16, body: &str) -> AuthorizerError {
    let server = Server::start(vec![json(status, body)]).await;
    answer(&server).await
}

async fn answer(server: &Server) -> AuthorizerError {
    let form = TokenForm::refresh_token(Secret::new(SECRET).expect("secret"));
    let client = FormTokenClient::new(
        http_client(Duration::from_secs(5)),
        server.endpoint("/token"),
    );
    client.fetch(&form).await.expect_err("refused")
}

#[tokio::test]
async fn given_a_status_that_waiting_cannot_fix_then_the_source_is_unavailable() {
    for status in [300, 301, 302, 304, 307, 400, 401, 403, 404, 410, 418, 499] {
        assert_eq!(
            refused(status, "{}").await,
            AuthorizerError::unavailable(format!("the token endpoint answered {status}")),
            "status {status}"
        );
    }
}

#[tokio::test]
async fn given_a_status_that_may_pass_then_the_fetch_failed() {
    for status in [429, 500, 502, 503, 504, 599] {
        assert_eq!(
            refused(status, "{}").await,
            AuthorizerError::fetch_failed(format!("the token endpoint answered {status}")),
            "status {status}"
        );
    }
}

#[tokio::test]
async fn given_an_error_member_of_lowercase_and_underscore_then_the_text_adds_it() {
    let body = r#"{"error":"invalid_grant","error_description":"AADSTS70000 FORM-SECRET-VALUE"}"#;
    let error = refused(400, body).await;
    assert_eq!(
        error,
        AuthorizerError::unavailable("the token endpoint answered 400: invalid_grant")
    );
    assert!(!format!("{error:?}").contains("AADSTS"));
    assert!(!format!("{error:?}").contains(SECRET));
}

#[tokio::test]
async fn given_an_error_code_with_an_unlisted_character_or_length_then_the_text_does_not_add_it() {
    let sixty_five = format!(r#"{{"error":"{}"}}"#, "a".repeat(65));
    let bodies = [
        r#"{"error":"Invalid_Grant"}"#.to_owned(),
        r#"{"error":"invalid-grant"}"#.to_owned(),
        r#"{"error":"invalid grant"}"#.to_owned(),
        r#"{"error":"invalid1"}"#.to_owned(),
        r#"{"error":""}"#.to_owned(),
        r#"{"error":7}"#.to_owned(),
        r#"{"error":{"error":"invalid_grant"}}"#.to_owned(),
        r#"["error"]"#.to_owned(),
        "not json".to_owned(),
        sixty_five,
    ];
    for body in bodies {
        assert_eq!(
            refused(400, &body).await,
            AuthorizerError::unavailable("the token endpoint answered 400"),
            "body {body}"
        );
    }
}

#[tokio::test]
async fn given_an_error_code_of_64_characters_then_the_text_adds_it() {
    let code = "a".repeat(64);
    let error = refused(400, &format!(r#"{{"error":"{code}"}}"#)).await;
    assert_eq!(
        error,
        AuthorizerError::unavailable(format!("the token endpoint answered 400: {code}"))
    );
}

#[tokio::test]
async fn given_a_server_error_with_an_error_member_then_the_text_adds_it_and_it_is_still_a_fetch_failure()
 {
    let error = refused(503, r#"{"error":"temporarily_unavailable"}"#).await;
    assert_eq!(
        error,
        AuthorizerError::fetch_failed("the token endpoint answered 503: temporarily_unavailable")
    );
}

#[tokio::test]
async fn given_a_refusal_whose_body_cannot_be_read_then_the_status_decides() {
    let server = Server::start(vec![short_body(400, 100, br#"{"error":"invalid_gr"#)]).await;
    assert_eq!(
        answer(&server).await,
        AuthorizerError::unavailable("the token endpoint answered 400")
    );
}

#[tokio::test]
async fn given_a_refusal_with_a_body_over_64_kib_then_the_error_member_is_not_read() {
    let body = format!(
        r#"{{"error":"invalid_grant","pad":"{}"}}"#,
        "x".repeat(70_000)
    );
    assert_eq!(
        refused(400, &body).await,
        AuthorizerError::unavailable("the token endpoint answered 400")
    );
}
