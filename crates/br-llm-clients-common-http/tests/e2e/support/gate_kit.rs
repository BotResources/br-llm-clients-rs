use std::time::Duration;

use br_llm_clients_common_http::{HttpClient, NoAuthorizer, SendError};
use hyper::Response;

use super::h2_server::{Options, TlsServer, service};
use super::pki::Pki;
use super::script_body::{ScriptBody, Step};
use super::{config, post};

pub fn trusting(connect: Duration, headers: Duration) -> HttpClient {
    let mut knobs = config();
    knobs.connect_timeout = connect;
    knobs.headers_timeout = headers;
    knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
    knobs.client()
}

pub fn answering(delay: Duration) -> crate::support::h2_server::Service {
    service(move |parts| async move {
        let hold = parts.headers.contains_key("x-hold");
        tokio::time::sleep(delay).await;
        let steps = if hold {
            vec![Step::Data(b"x"), Step::Stall]
        } else {
            vec![Step::Data(b"ok")]
        };
        Ok(Response::new(ScriptBody::new(steps)))
    })
}

pub async fn run(
    client: &HttpClient,
    server: &TlsServer,
    count: usize,
) -> Vec<Result<(), SendError>> {
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..count {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        tasks.spawn(async move {
            let mut response = client.send(post(&endpoint, "a"), &NoAuthorizer).await?;
            response
                .body_mut()
                .collect(10)
                .await
                .map(|_| ())
                .map_err(|_| SendError::Protocol {
                    text: "body".to_owned(),
                })
        });
    }
    let mut outcomes = Vec::new();
    while let Some(outcome) = tasks.join_next().await {
        outcomes.push(outcome.expect("join"));
    }
    outcomes
}

pub fn options(max_streams: u32, handshake_delay: Duration, stall_first: usize) -> Options {
    Options {
        max_streams: Some(max_streams),
        handshake_delay,
        stall_first,
    }
}

pub async fn until_accepted(server: &TlsServer, count: usize) {
    let started = std::time::Instant::now();
    while server.accepted() < count {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "only {} of {count} connections were accepted",
            server.accepted()
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
