use std::time::Duration;

use br_llm_clients_common_auth::TokenAuthorizer;
use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, Request, SendError};
use tokio::time::{Instant, advance};

use crate::support::cache_of;
use crate::support::source::{Scripted, Step};

const TOTAL: Duration = Duration::from_millis(100);

fn client() -> HttpClient {
    let config = HttpConfig::new(
        Duration::from_secs(5),
        Duration::from_secs(5),
        Duration::from_secs(5),
    )
    .with_total_timeout(TOTAL);
    HttpClient::new(config).expect("client")
}

fn request() -> Request {
    let endpoint = Endpoint::plain_http("http://127.0.0.1:9/v1").expect("endpoint");
    Request::new(endpoint, b"{}".to_vec())
}

#[tokio::test(start_paused = true)]
async fn given_a_slow_token_fetch_when_the_total_timeout_is_shorter_then_the_call_ends_with_a_total_timeout()
 {
    let source = Scripted::new(vec![Step::hang()]);
    let authorizer = TokenAuthorizer::bearer(cache_of(&source));
    let start = Instant::now();
    let failure = client()
        .send(request(), &authorizer)
        .await
        .expect_err("timeout");
    assert!(matches!(failure, SendError::TotalTimeout { .. }));
    assert_eq!(start.elapsed(), TOTAL);
    assert_eq!(source.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn given_a_usable_token_when_the_leader_waits_for_a_slow_fetch_then_the_total_timeout_ends_the_call()
 {
    let source = Scripted::new(vec![Step::secs("old", 1000), Step::hang()]);
    let cache = cache_of(&source);
    cache.token().await.expect("first");
    advance(Duration::from_secs(800)).await;
    let authorizer = TokenAuthorizer::bearer(cache);
    let start = Instant::now();
    let failure = client()
        .send(request(), &authorizer)
        .await
        .expect_err("timeout");
    assert!(matches!(failure, SendError::TotalTimeout { .. }));
    assert_eq!(start.elapsed(), TOTAL);
    assert_eq!(source.calls(), 2);
}
