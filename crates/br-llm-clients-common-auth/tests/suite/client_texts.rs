use std::time::Duration;

use br_llm_clients_common_auth::{FormTokenClient, TokenForm};
use br_llm_clients_common_http::{AuthorizerError, Secret};

use crate::support::server::{Server, http_client, json, response};

const FORM_SECRET: &str = "FORM-SECRET-41";
const ACCESS: &str = "ACCESS-TOKEN-42";
const REFRESH: &str = "REFRESH-TOKEN-43";

fn hidden() -> Vec<Secret> {
    [
        FORM_SECRET,
        ACCESS,
        REFRESH,
        "PLAIN-VALUE-44",
        "HEADER-VALUE-45",
    ]
    .into_iter()
    .map(|value| Secret::new(value).expect("secret"))
    .collect()
}

async fn error_for(status: u16, body: &str) -> AuthorizerError {
    let server = Server::start(vec![response(
        status,
        &[("x-echo", "HEADER-VALUE-45")],
        body.as_bytes(),
    )])
    .await;
    let form =
        TokenForm::client_credentials("PLAIN-VALUE-44", Secret::new(FORM_SECRET).expect("secret"))
            .expect("form");
    FormTokenClient::new(
        http_client(Duration::from_secs(5)),
        server.endpoint("/token"),
    )
    .fetch(&form)
    .await
    .expect_err("error")
}

fn assert_clean(error: &AuthorizerError) {
    let texts = [error.to_string(), format!("{error:?}")];
    for secret in hidden() {
        for text in &texts {
            assert!(!secret.appears_in(text), "{text}");
        }
    }
}

#[tokio::test]
async fn given_any_answer_that_echoes_the_values_then_no_error_text_holds_one() {
    let echo = format!(
        r#"{{"error":"invalid_grant","error_description":"{FORM_SECRET} {ACCESS}","access_token":"{ACCESS}","refresh_token":"{REFRESH}","client_id":"PLAIN-VALUE-44"}}"#
    );
    let malformed = [
        format!(r#"{{"access_token":"{ACCESS}","token_type":"mac","refresh_token":"{REFRESH}"}}"#),
        format!(r#"{{"access_token":"{ACCESS}","expires_in":"{FORM_SECRET}"}}"#),
        format!(r#"{{"access_token":"{ACCESS}","refresh_token":5}}"#),
        format!(r#"["{ACCESS}"]"#),
    ];
    for status in [302, 400, 401, 429, 500] {
        assert_clean(&error_for(status, &echo).await);
    }
    for body in malformed {
        assert_clean(&error_for(200, &body).await);
    }
}

async fn status_text(form: &TokenForm, body: &str) -> AuthorizerError {
    let server = Server::start(vec![json(400, body)]).await;
    FormTokenClient::new(
        http_client(Duration::from_secs(5)),
        server.endpoint("/token"),
    )
    .fetch(form)
    .await
    .expect_err("error")
}

#[tokio::test]
async fn given_an_error_member_that_holds_a_secret_of_the_form_then_the_text_drops_it() {
    let secret = Secret::new("hunter_two_lowercase").expect("secret");
    let form = TokenForm::client_credentials("app", secret.clone()).expect("form");
    for body in [
        r#"{"error":"hunter_two_lowercase"}"#,
        r#"{"error":"bad_hunter_two_lowercase_given"}"#,
    ] {
        let error = status_text(&form, body).await;
        assert_eq!(
            error,
            AuthorizerError::unavailable("the token endpoint answered 400"),
            "{body}"
        );
        assert!(!secret.appears_in(&error.to_string()));
    }
}

#[tokio::test]
async fn given_an_error_member_equal_to_a_plain_value_of_the_form_then_the_text_drops_it() {
    let form =
        TokenForm::client_credentials("my_app", Secret::new("X-1").expect("secret")).expect("form");
    assert_eq!(
        status_text(&form, r#"{"error":"my_app"}"#).await,
        AuthorizerError::unavailable("the token endpoint answered 400")
    );
    assert_eq!(
        status_text(&form, r#"{"error":"client_credentials"}"#).await,
        AuthorizerError::unavailable("the token endpoint answered 400")
    );
}

#[tokio::test]
async fn given_an_error_member_that_only_contains_a_plain_value_of_the_form_then_the_text_keeps_it()
{
    let form = TokenForm::refresh_token(Secret::new("RT-SECRET-1").expect("secret"));
    assert_eq!(
        status_text(&form, r#"{"error":"invalid_refresh_token"}"#).await,
        AuthorizerError::unavailable("the token endpoint answered 400: invalid_refresh_token")
    );
}

#[tokio::test]
async fn given_an_error_member_that_is_a_part_of_a_secret_of_the_form_then_the_text_drops_it() {
    let secret = Secret::new("s".repeat(70)).expect("secret");
    let form = TokenForm::client_credentials("app", secret).expect("form");
    let echoed = format!(r#"{{"error":"{}"}}"#, "s".repeat(64));
    assert_eq!(
        status_text(&form, &echoed).await,
        AuthorizerError::unavailable("the token endpoint answered 400")
    );
}
