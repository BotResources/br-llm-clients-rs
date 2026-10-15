use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use hyper::Response;

use crate::authorizers::NoAuthorizer;
use crate::client::HttpClient;
use crate::support::h2_server::{Options, TlsServer, service};
use crate::support::pki::Pki;
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{config, post};
use crate::tuning::Tuning;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_burst_against_a_one_stream_server_then_never_more_than_sixteen_connections_are_pooled()
 {
    let pki = Pki::trusted();
    let server = TlsServer::start_with(
        pki.acceptor(&[b"h2"]),
        service(|_| async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            Ok(Response::new(ScriptBody::new([Step::Data(b"ok")])))
        }),
        Options {
            max_streams: Some(1),
            handshake_delay: Duration::ZERO,
            stall_first: 0,
        },
    )
    .await;
    let mut knobs = config();
    knobs.extra_root_certificates = vec![pki.ca_pem.clone()];
    let client = HttpClient::with_tuning(knobs.build(), Tuning::FIXED).expect("client");
    let peak = Arc::new(AtomicUsize::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let sampler = {
        let (client, peak, running) = (client.clone(), Arc::clone(&peak), Arc::clone(&running));
        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                peak.fetch_max(client.h2_connections(), Ordering::SeqCst);
                tokio::task::yield_now().await;
            }
        })
    };
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..80 {
        let (client, endpoint) = (client.clone(), server.endpoint("/"));
        tasks.spawn(async move {
            let mut response = client.send(post(&endpoint, "a"), &NoAuthorizer).await?;
            response
                .body_mut()
                .collect(10)
                .await
                .map(|_| ())
                .map_err(|_| crate::send_error::SendError::Protocol {
                    text: "body".to_owned(),
                })
        });
    }
    while let Some(outcome) = tasks.join_next().await {
        assert!(outcome.expect("join").is_ok());
    }
    running.store(false, Ordering::SeqCst);
    sampler.await.expect("sampler");
    assert!(peak.load(Ordering::SeqCst) >= 2);
    assert!(
        peak.load(Ordering::SeqCst) <= 16,
        "{} pooled",
        peak.load(Ordering::SeqCst)
    );
}
