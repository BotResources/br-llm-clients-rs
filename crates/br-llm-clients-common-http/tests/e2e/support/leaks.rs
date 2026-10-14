use br_llm_clients_common_http::{BodyError, Endpoint, HttpClient, Request, SendError};

use super::guarded;
use super::spy::Spy;

pub const CREDENTIAL: &str = "sk-CRED-9137";
pub const REQUEST_HEADER: &str = "reqhdr-VAL-7741";
pub const REQUEST_BODY: &str = "req-BODY-5503";
pub const QUERY: &str = "qry-SECRET-3319";
pub const RESPONSE_HEADER: &str = "resp-HDR-4408";
pub const RESPONSE_BODY: &str = "resp-BODY-2260";

pub fn forbidden() -> [&'static str; 6] {
    [
        CREDENTIAL,
        REQUEST_HEADER,
        REQUEST_BODY,
        QUERY,
        RESPONSE_HEADER,
        RESPONSE_BODY,
    ]
}

pub fn assert_clean(label: &str, texts: &[String]) {
    for text in texts {
        for secret in forbidden() {
            assert!(!text.contains(secret), "{label} leaks {secret}: {text}");
        }
    }
}

pub fn texts_of_send(error: &SendError) -> Vec<String> {
    vec![
        error.to_string(),
        format!("{error:?}"),
        format!("{error:#?}"),
    ]
}

pub fn texts_of_body(error: &BodyError) -> Vec<String> {
    vec![
        error.to_string(),
        format!("{error:?}"),
        format!("{error:#?}"),
    ]
}

pub fn secret_request(endpoint: &Endpoint) -> Request {
    Request::new(endpoint.clone(), REQUEST_BODY)
        .with_header("x-request", REQUEST_HEADER)
        .expect("header")
}

pub fn with_query(text: &str) -> String {
    format!("{text}?token={QUERY}&k={CREDENTIAL}")
}

pub fn plain(port: u16) -> Endpoint {
    Endpoint::plain_http(&with_query(&format!("http://127.0.0.1:{port}/p"))).expect("endpoint")
}

pub fn credential_spy() -> Spy {
    Spy::returning(&[("authorization", CREDENTIAL)])
}

pub async fn send_failure(client: &HttpClient, endpoint: &Endpoint) -> SendError {
    guarded(client.send(secret_request(endpoint), &credential_spy()))
        .await
        .unwrap_err()
}

pub async fn body_failure(client: &HttpClient, endpoint: &Endpoint) -> BodyError {
    let mut response = guarded(client.send(secret_request(endpoint), &credential_spy()))
        .await
        .expect("response");
    loop {
        match guarded(response.body_mut().next_chunk()).await {
            Ok(Some(_)) => {}
            Ok(None) => panic!("the stream ended without a failure"),
            Err(error) => return error,
        }
    }
}
