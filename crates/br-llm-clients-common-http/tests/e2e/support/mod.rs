pub mod canned;
pub mod frames;
pub mod gate_kit;
pub mod h2_headers;
pub mod h2_server;
pub mod leaks;
pub mod pki;
pub mod probe;
pub mod proxy;
pub mod raw;
pub mod raw_h2;
pub mod script_body;
pub mod spy;

use std::future::Future;
use std::time::Duration;

use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, Request};

pub const LONG: Duration = Duration::from_secs(20);

pub struct Knobs {
    pub connect_timeout: Duration,
    pub headers_timeout: Duration,
    pub idle_timeout: Duration,
    pub total_timeout: Option<Duration>,
    pub user_agent: Option<String>,
    pub extra_root_certificates: Vec<String>,
}

impl Knobs {
    pub fn build(self) -> HttpConfig {
        let mut config = HttpConfig::new(
            self.connect_timeout,
            self.headers_timeout,
            self.idle_timeout,
        )
        .with_extra_root_certificates(self.extra_root_certificates);
        if let Some(total) = self.total_timeout {
            config = config.with_total_timeout(total);
        }
        if let Some(agent) = self.user_agent {
            config = config.with_user_agent(agent);
        }
        config
    }

    pub fn client(self) -> HttpClient {
        HttpClient::new(self.build()).expect("client")
    }
}

pub fn config() -> Knobs {
    Knobs {
        connect_timeout: LONG,
        headers_timeout: LONG,
        idle_timeout: LONG,
        total_timeout: None,
        user_agent: None,
        extra_root_certificates: Vec::new(),
    }
}

pub fn client() -> HttpClient {
    config().client()
}

pub fn client_with(change: impl FnOnce(&mut Knobs)) -> HttpClient {
    let mut knobs = config();
    change(&mut knobs);
    knobs.client()
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
    assert_port_one_is_closed();
    let text = format!(
        "{}://127.0.0.1:1/closed",
        if https { "https" } else { "http" }
    );
    if https {
        Endpoint::https(&text).expect("endpoint")
    } else {
        Endpoint::plain_http(&text).expect("endpoint")
    }
}

pub fn assert_port_one_is_closed() {
    let address = "127.0.0.1:1".parse().expect("address");
    let attempt = std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2));
    assert!(
        matches!(&attempt, Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused),
        "this host listens on or filters 127.0.0.1:1, so the refused-connection tests cannot run: {attempt:?}"
    );
}
