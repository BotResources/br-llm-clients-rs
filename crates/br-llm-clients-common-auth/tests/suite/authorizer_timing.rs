use std::time::Duration;

use br_llm_clients_common_auth::TokenAuthorizer;
use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, Request, SendError};

use crate::support::cache_of;
use crate::support::source::{Scripted, Step};

#[tokio::test]
async fn given_a_slow_token_fetch_when_the_total_timeout_is_shorter_then_the_call_ends_with_a_total_timeout()
 {
    let source = Scripted::new(vec![Step::hang()]);
    let authorizer = TokenAuthorizer::bearer(cache_of(&source));
    let config = HttpConfig::new(
        Duration::from_secs(5),
        Duration::from_secs(5),
        Duration::from_secs(5),
    )
    .with_total_timeout(Duration::from_millis(100));
    let client = HttpClient::new(config).expect("client");
    let endpoint = Endpoint::plain_http("http://127.0.0.1:9/v1").expect("endpoint");
    let failure = client
        .send(Request::new(endpoint, b"{}".to_vec()), &authorizer)
        .await
        .expect_err("timeout");
    assert!(matches!(failure, SendError::TotalTimeout { .. }));
}
