use std::sync::Arc;

use br_llm_clients_common_auth::{
    AccessToken, AuthConfigError, FormTokenClient, StaticToken, TokenAuthorizer, TokenCache,
    TokenForm, TokenReply, TokenSource,
};

fn send_sync<T: Send + Sync>() {}

fn send<T: Send>(_: &T) {}

fn clone<T: Clone>() {}

#[test]
fn given_the_public_types_then_they_are_send_and_sync() {
    send_sync::<AccessToken>();
    send_sync::<StaticToken>();
    send_sync::<TokenCache>();
    send_sync::<TokenAuthorizer>();
    send_sync::<TokenForm>();
    send_sync::<TokenReply>();
    send_sync::<FormTokenClient>();
    send_sync::<AuthConfigError>();
    send_sync::<Arc<dyn TokenSource>>();
}

#[test]
fn given_the_types_the_library_says_can_be_cloned_then_they_are_clone() {
    clone::<AccessToken>();
    clone::<TokenCache>();
    clone::<TokenAuthorizer>();
    clone::<TokenForm>();
    clone::<TokenReply>();
    clone::<FormTokenClient>();
    clone::<AuthConfigError>();
}

#[tokio::test]
async fn given_the_futures_of_the_library_then_they_are_send() {
    let cache = TokenCache::new(Arc::new(StaticToken::new(
        br_llm_clients_common_http::Secret::new("t").expect("secret"),
    )));
    let token = cache.token();
    send(&token);
    token.await.expect("token");
    let endpoint =
        br_llm_clients_common_http::Endpoint::plain_http("http://127.0.0.1:9/t").expect("endpoint");
    let config = br_llm_clients_common_http::HttpConfig::new(
        std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(1),
    );
    let client = FormTokenClient::new(
        br_llm_clients_common_http::HttpClient::new(config).expect("http"),
        endpoint,
    );
    let form = TokenForm::new();
    let fetch = client.fetch(&form);
    send(&fetch);
}
