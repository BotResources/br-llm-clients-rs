use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_auth::{
    AccessToken, AuthConfigError, FormTokenClient, StaticToken, TokenAuthorizer, TokenCache,
    TokenForm, TokenReply,
};
use br_llm_clients_common_http::{Endpoint, Secret};

use crate::support::cache_of;
use crate::support::server::http_client;
use crate::support::source::{Scripted, Step};

const VALUE: &str = "CREDENTIAL-VALUE-123";

fn secret() -> Secret {
    Secret::new(VALUE).expect("secret")
}

#[test]
fn given_an_access_token_then_debug_shows_a_placeholder_and_the_validity() {
    let token = AccessToken::new(secret(), Some(Duration::from_secs(60)));
    let shown = format!("{token:?}");
    assert!(!token.secret().appears_in(&shown));
    assert_eq!(
        shown,
        "AccessToken { secret: [redacted], valid_for: Some(60s) }"
    );
    let unknown = format!("{:?}", AccessToken::new(secret(), None));
    assert_eq!(
        unknown,
        "AccessToken { secret: [redacted], valid_for: None }"
    );
}

#[test]
fn given_an_access_token_then_its_parts_are_read_and_a_clone_holds_the_same() {
    let token = AccessToken::new(secret(), Some(Duration::ZERO));
    let copy = token.clone();
    assert_eq!(copy.secret().expose(), VALUE);
    assert_eq!(copy.valid_for(), Some(Duration::ZERO));
}

#[test]
fn given_a_static_token_then_debug_shows_no_value() {
    let source = StaticToken::new(secret());
    assert!(!secret().appears_in(&format!("{source:?}")));
}

#[tokio::test]
async fn given_a_cache_that_holds_a_token_then_debug_of_the_cache_and_the_authorizer_show_no_token()
{
    let source = Scripted::new(vec![Step::token(VALUE, Some(Duration::from_secs(1000)))]);
    let cache = cache_of(&source);
    let token = cache.token().await.expect("token");
    assert!(!token.appears_in(&format!("{cache:?}")));
    let authorizer = TokenAuthorizer::bearer(cache).with_token_observer(Arc::new(|_| {}));
    assert!(!token.appears_in(&format!("{authorizer:?}")));
    let static_cache = TokenCache::new(Arc::new(StaticToken::new(secret())));
    let token = static_cache.token().await.expect("token");
    assert!(!token.appears_in(&format!("{static_cache:?}")));
}

#[test]
fn given_a_form_then_debug_shows_the_names_and_never_a_value() {
    let form = TokenForm::client_credentials("PUBLIC-CLIENT-ID", secret())
        .expect("form")
        .plain("scope", "PLAIN-VALUE")
        .expect("scope");
    let shown = format!("{form:?}");
    assert_eq!(
        shown,
        r#"TokenForm { fields: ["grant_type", "client_id", "client_secret", "scope"] }"#
    );
    assert!(!secret().appears_in(&shown));
}

#[test]
fn given_a_reply_then_debug_shows_no_token() {
    let refresh = Secret::new("REFRESH-VALUE-9").expect("secret");
    let reply = TokenReply {
        access_token: AccessToken::new(secret(), None),
        refresh_token: Some(refresh.clone()),
    };
    let shown = format!("{:?}", reply.clone());
    assert!(!secret().appears_in(&shown));
    assert!(!refresh.appears_in(&shown));
}

#[test]
fn given_a_client_then_debug_shows_the_endpoint_without_its_query() {
    let endpoint =
        Endpoint::plain_http("http://127.0.0.1:9/token?key=QUERY-SECRET").expect("endpoint");
    let client = FormTokenClient::new(http_client(Duration::from_secs(1)), endpoint.clone());
    let shown = format!("{client:?}");
    assert!(shown.contains(&format!("{endpoint:?}")));
    assert!(!shown.contains("QUERY-SECRET"));
}

#[test]
fn given_a_refused_form_field_then_the_error_text_holds_no_value() {
    let errors: Vec<AuthConfigError> = vec![
        TokenForm::new().plain("", VALUE).expect_err("empty"),
        TokenForm::new()
            .secret("name", secret())
            .and_then(|form| form.secret("name", secret()))
            .expect_err("twice"),
    ];
    for error in errors {
        assert!(!secret().appears_in(&error.to_string()));
        assert!(!secret().appears_in(&format!("{error:?}")));
    }
}
