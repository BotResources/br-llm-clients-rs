use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use hyper::Response;

use crate::authorizers::NoAuthorizer;
use crate::client::HttpClient;
use crate::pool::MAX_H2_PER_HOST;
use crate::send_error::SendError;
use crate::support::h2_server::{Options, TlsServer, service};
use crate::support::pki::Pki;
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{config, guarded, post};
use crate::tuning::Tuning;

struct Flip {
    outcomes: Vec<Result<(), SendError>>,
    peak: usize,
    accepted: usize,
    closed: usize,
}

async fn send_one(client: HttpClient, server: &TlsServer) -> Result<(), SendError> {
    let mut response = client
        .send(post(&server.endpoint("/"), "a"), &NoAuthorizer)
        .await?;
    response
        .body_mut()
        .collect(10)
        .await
        .map(|_| ())
        .map_err(|_| SendError::Protocol {
            text: "body".to_owned(),
        })
}

async fn burst_after_one_http1_connection(sends: usize) -> Flip {
    let pki = Pki::trusted();
    let server = Arc::new(
        TlsServer::start_with(
            pki.acceptor(&[b"h2"]),
            service(|_| async {
                tokio::time::sleep(Duration::from_millis(600)).await;
                Ok(Response::new(ScriptBody::new([Step::Data(b"ok")])))
            }),
            Options {
                max_streams: Some(100),
                handshake_delay: Duration::from_millis(100),
                stall_first: 0,
                http1_first: 1,
            },
        )
        .await,
    );
    let mut knobs = config();
    knobs.extra_root_certificates = vec![pki.ca_pem.clone()];
    let client = HttpClient::with_tuning(knobs.build(), Tuning::FIXED).expect("client");
    guarded(send_one(client.clone(), &server))
        .await
        .expect("the warm-up send");
    assert_eq!(client.h2_connections(), 0);
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
    for _ in 0..sends {
        let (client, server) = (client.clone(), Arc::clone(&server));
        tasks.spawn(async move { send_one(client, &server).await });
    }
    let mut outcomes = Vec::new();
    while let Some(outcome) = guarded(tasks.join_next()).await {
        outcomes.push(outcome.expect("join"));
    }
    running.store(false, Ordering::SeqCst);
    sampler.await.expect("sampler");
    let accepted = server.accepted();
    let mut closed = server.closed();
    for _ in 0..200 {
        if accepted.saturating_sub(closed) <= MAX_H2_PER_HOST.saturating_add(1) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
        closed = server.closed();
    }
    Flip {
        outcomes,
        peak: peak.load(Ordering::SeqCst),
        accepted,
        closed,
    }
}

fn assert_capped(flip: &Flip, sends: usize) {
    assert_eq!(flip.outcomes.len(), sends);
    assert!(
        flip.outcomes.iter().all(Result::is_ok),
        "{:?}",
        flip.outcomes
    );
    assert_eq!(
        flip.peak, MAX_H2_PER_HOST,
        "{} HTTP/2 connections were pooled at the peak",
        flip.peak
    );
    assert!(
        flip.accepted.saturating_sub(flip.closed) <= MAX_H2_PER_HOST.saturating_add(1),
        "{} connections stay open on the server",
        flip.accepted.saturating_sub(flip.closed)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_host_that_answered_http1_and_then_answers_http2_when_twenty_sends_dial_together_then_sixteen_http2_connections_are_pooled()
 {
    assert_capped(&burst_after_one_http1_connection(20).await, 20);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_host_that_answered_http1_and_then_answers_http2_when_sixty_sends_dial_together_then_sixteen_http2_connections_are_pooled()
 {
    assert_capped(&burst_after_one_http1_connection(60).await, 60);
}
