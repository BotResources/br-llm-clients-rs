use std::time::Duration;

use br_llm_clients_common_http::{HttpClient, NoAuthorizer};
use hyper::Response;

use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{config, post};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_many_cold_bursts_then_each_burst_opens_exactly_one_http2_connection() {
    for round in 0..150 {
        let server = TlsServer::start_limited(
            Pki::trusted().acceptor(&[b"h2"]),
            service(|_| async { Ok(Response::new(ScriptBody::new(vec![Step::Data(b"ok")]))) }),
            Some(1000),
            Duration::ZERO,
        )
        .await;
        let mut knobs = config();
        knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
        let client: HttpClient = knobs.client();
        let mut tasks = tokio::task::JoinSet::new();
        for i in 0..200 {
            let (client, endpoint) = (client.clone(), server.endpoint("/"));
            tasks.spawn(async move {
                let mut response = client.send(post(&endpoint, "a"), &NoAuthorizer).await?;
                response
                    .body_mut()
                    .collect(10)
                    .await
                    .map(|_| i)
                    .map_err(|_| br_llm_clients_common_http::SendError::Protocol {
                        text: "body".into(),
                    })
            });
        }
        let mut errors = Vec::new();
        while let Some(done) = tasks.join_next().await {
            if let Err(error) = done.expect("join") {
                errors.push(format!("{error:?}"));
            }
        }
        assert!(errors.is_empty(), "round {round}: {errors:?}");
        assert_eq!(server.accepted(), 1, "round {round}");
    }
}
