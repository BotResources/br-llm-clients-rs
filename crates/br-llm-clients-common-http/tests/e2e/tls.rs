use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_common_http::{HttpClient, NoAuthorizer, SendError};
use hyper::{Response, Version};

use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{client, config, guarded, post, trusting_client};

fn versions_server_service(
    versions: Arc<std::sync::Mutex<Vec<Version>>>,
) -> crate::support::h2_server::Service {
    service(move |req| {
        versions.lock().expect("lock").push(req.version());
        async { Ok(Response::new(ScriptBody::new([Step::Data(b"secure")]))) }
    })
}

#[tokio::test]
async fn given_a_server_with_a_certificate_from_an_extra_root_then_https_works_over_http1() {
    let pki = Pki::trusted();
    let versions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let server = TlsServer::start(
        pki.acceptor(&[b"http/1.1"]),
        versions_server_service(Arc::clone(&versions)),
    )
    .await;
    let mut response = guarded(
        trusting_client(&pki.ca_pem).send(post(&server.endpoint("/x"), "a"), &NoAuthorizer),
    )
    .await
    .expect("response");
    assert_eq!(
        response.body_mut().collect(100).await.expect("body").bytes,
        b"secure"
    );
    assert_eq!(
        versions.lock().expect("lock").as_slice(),
        [Version::HTTP_11]
    );
}

#[tokio::test]
async fn given_a_server_offering_h2_then_http2_is_negotiated() {
    let pki = Pki::trusted();
    let versions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let server = TlsServer::start(
        pki.acceptor(&[b"h2", b"http/1.1"]),
        versions_server_service(Arc::clone(&versions)),
    )
    .await;
    let mut response = guarded(
        trusting_client(&pki.ca_pem).send(post(&server.endpoint("/x"), "a"), &NoAuthorizer),
    )
    .await
    .expect("response");
    assert_eq!(
        response.body_mut().collect(100).await.expect("body").bytes,
        b"secure"
    );
    assert_eq!(versions.lock().expect("lock").as_slice(), [Version::HTTP_2]);
}

#[tokio::test]
async fn given_a_server_without_alpn_then_http1_is_used() {
    let pki = Pki::trusted();
    let versions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let server = TlsServer::start(
        pki.acceptor(&[]),
        versions_server_service(Arc::clone(&versions)),
    )
    .await;
    guarded(trusting_client(&pki.ca_pem).send(post(&server.endpoint("/x"), "a"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        versions.lock().expect("lock").as_slice(),
        [Version::HTTP_11]
    );
}

#[tokio::test]
async fn given_a_certificate_from_an_unknown_authority_then_it_is_refused_with_tls() {
    let pki = Pki::other_ca();
    let versions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let server = TlsServer::start(
        pki.acceptor(&[b"http/1.1"]),
        versions_server_service(Arc::clone(&versions)),
    )
    .await;
    let error = guarded(client().send(post(&server.endpoint("/x"), "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Tls { .. }), "{error:?}");
    assert!(versions.lock().expect("lock").is_empty());
}

#[tokio::test]
async fn given_a_root_that_is_not_the_servers_then_the_certificate_is_refused_with_tls() {
    let served = Pki::other_ca();
    let trusted = Pki::trusted();
    let versions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let server = TlsServer::start(
        served.acceptor(&[b"http/1.1"]),
        versions_server_service(Arc::clone(&versions)),
    )
    .await;
    let error = guarded(
        trusting_client(&trusted.ca_pem).send(post(&server.endpoint("/x"), "a"), &NoAuthorizer),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, SendError::Tls { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_certificate_for_another_name_then_it_is_refused_with_tls() {
    let pki = Pki::wrong_name();
    let versions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let server = TlsServer::start(
        pki.acceptor(&[b"http/1.1"]),
        versions_server_service(Arc::clone(&versions)),
    )
    .await;
    let error = guarded(
        trusting_client(&pki.ca_pem).send(post(&server.endpoint("/x"), "a"), &NoAuthorizer),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, SendError::Tls { .. }), "{error:?}");
}

#[tokio::test]
async fn given_a_refused_certificate_then_the_request_and_its_credential_are_never_sent() {
    let pki = Pki::other_ca();
    let seen = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&seen);
    let server = TlsServer::start(
        pki.acceptor(&[b"http/1.1"]),
        service(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            async { Ok(Response::new(ScriptBody::new([]))) }
        }),
    )
    .await;
    let error = guarded(client().send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Tls { .. }));
    assert_eq!(seen.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn given_two_extra_roots_then_either_one_is_trusted() {
    let first = Pki::trusted();
    let second = Pki::other_ca();
    let mut config = config();
    config.extra_root_certificates = vec![first.ca_pem.clone(), second.ca_pem.clone()];
    let client = HttpClient::new(config).expect("client");
    for pki in [first, second] {
        let versions = Arc::new(std::sync::Mutex::new(Vec::new()));
        let server = TlsServer::start(
            pki.acceptor(&[b"http/1.1"]),
            versions_server_service(versions),
        )
        .await;
        let response = guarded(client.send(post(&server.endpoint("/"), "a"), &NoAuthorizer))
            .await
            .expect("response");
        assert_eq!(response.status(), 200);
    }
}
