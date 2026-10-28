use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_common_auth::{
    AccessToken, FormTokenClient, StaticToken, TokenAuthorizer, TokenCache, TokenForm, TokenReply,
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
    assert!(!shown.contains(VALUE));
    assert!(shown.contains("[redacted]"));
    assert!(shown.contains("60s"));
}

#[test]
fn given_a_static_token_then_debug_shows_no_value() {
    assert!(!format!("{:?}", StaticToken::new(secret())).contains(VALUE));
}

#[tokio::test]
async fn given_a_cache_that_holds_a_token_then_debug_of_the_cache_and_the_authorizer_show_no_token()
{
    let source = Scripted::new(vec![Step::token(VALUE, Some(Duration::from_secs(1000)))]);
    let cache = cache_of(&source);
    cache.token().await.expect("token");
    assert!(!format!("{cache:?}").contains(VALUE));
    assert!(!format!("{:?}", TokenAuthorizer::bearer(cache)).contains(VALUE));
    let static_cache = TokenCache::new(Arc::new(StaticToken::new(secret())));
    static_cache.token().await.expect("token");
    assert!(!format!("{static_cache:?}").contains(VALUE));
}

#[test]
fn given_a_form_then_debug_shows_the_names_and_never_a_value() {
    let form = TokenForm::client_credentials("PUBLIC-CLIENT-ID", secret())
        .expect("form")
        .plain("scope", "PLAIN-VALUE")
        .expect("scope");
    let shown = format!("{form:?}");
    for name in ["grant_type", "client_id", "client_secret", "scope"] {
        assert!(shown.contains(name), "{name}");
    }
    for value in [
        VALUE,
        "PUBLIC-CLIENT-ID",
        "PLAIN-VALUE",
        "client_credentials",
    ] {
        assert!(!shown.contains(value), "{value}");
    }
}

#[test]
fn given_a_reply_then_debug_shows_no_token() {
    let reply = TokenReply {
        access_token: AccessToken::new(secret(), None),
        refresh_token: Some(Secret::new("REFRESH-VALUE-9").expect("secret")),
    };
    let shown = format!("{reply:?}");
    assert!(!shown.contains(VALUE));
    assert!(!shown.contains("REFRESH-VALUE-9"));
}

#[test]
fn given_a_client_then_debug_shows_the_endpoint_without_its_query() {
    let endpoint =
        Endpoint::plain_http("http://127.0.0.1:9/token?key=QUERY-SECRET").expect("endpoint");
    let client = FormTokenClient::new(http_client(Duration::from_secs(1)), endpoint);
    let shown = format!("{client:?}");
    assert!(shown.contains("/token"));
    assert!(!shown.contains("QUERY-SECRET"));
}
