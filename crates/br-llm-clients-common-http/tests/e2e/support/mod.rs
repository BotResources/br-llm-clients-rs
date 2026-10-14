pub mod canned;
pub mod h2_server;
pub mod leaks;
pub mod pki;
pub mod raw;
pub mod script_body;
pub mod spy;

use std::future::Future;
use std::time::Duration;

use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, Request};

pub const LONG: Duration = Duration::from_secs(20);

pub fn config() -> HttpConfig {
    HttpConfig::new(LONG, LONG, LONG)
}

pub fn client() -> HttpClient {
    HttpClient::new(config()).expect("client")
}

pub fn client_with(change: impl FnOnce(&mut HttpConfig)) -> HttpClient {
    let mut config = config();
    change(&mut config);
    HttpClient::new(config).expect("client")
}

pub fn trusting_client(ca_pem: &str) -> HttpClient {
    client_with(|c| c.extra_root_certificates = vec![ca_pem.to_owned()])
}

pub fn post(endpoint: &Endpoint, body: &str) -> Request {
    Request::new(endpoint.clone(), body)
}

pub async fn guarded<F: Future>(work: F) -> F::Output {
    tokio::time::timeout(Duration::from_secs(30), work)
        .await
        .expect("the test hung")
}

pub fn closed_port_endpoint(https: bool) -> Endpoint {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let text = format!(
        "{}://127.0.0.1:{port}/closed",
        if https { "https" } else { "http" }
    );
    if https {
        Endpoint::https(&text).expect("endpoint")
    } else {
        Endpoint::plain_http(&text).expect("endpoint")
    }
}
