use std::time::Duration;

use br_llm_clients_common_http::{BodyError, HttpClient, NoAuthorizer, SendError};
use hyper::Response;

use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::raw_h2::{Announce, Answer, Ending, Policy, RawH2, TlsFault};
use crate::support::script_body::{ScriptBody, Step};
use crate::support::{Knobs, client_with, closed_port_endpoint, post};

const SHORT: Duration = Duration::from_millis(200);
const CONNECT: Duration = Duration::from_millis(500);
const TOTAL: Duration = Duration::from_millis(500);

fn trusting_with(change: impl FnOnce(&mut Knobs)) -> HttpClient {
    client_with(|knobs| {
        knobs.extra_root_certificates = vec![Pki::trusted().ca_pem.clone()];
        change(knobs);
    })
}

fn unanswered() -> Policy {
    Policy {
        answer: Answer::Never,
        ..Policy::healthy()
    }
}

async fn send_error(client: &HttpClient, server: &RawH2) -> SendError {
    client
        .send(post(&server.endpoint(), "x"), &NoAuthorizer)
        .await
        .unwrap_err()
}

fn assert_names_no_endpoint(text: &str, server: &RawH2) {
    assert!(!text.contains("127.0.0.1"), "{text}");
    assert!(!text.contains(&server.addr.port().to_string()), "{text}");
}

#[tokio::test]
async fn given_a_refused_connection_then_the_text_is_the_reason_the_kind_and_the_endpoint() {
    let client = trusting_with(|knobs| knobs.connect_timeout = Duration::from_secs(5));
    let endpoint = closed_port_endpoint(true);
    let error = client
        .send(post(&endpoint, "x"), &NoAuthorizer)
        .await
        .unwrap_err();
    let SendError::Connect { text } = error else {
        panic!("{error:?}");
    };
    assert_eq!(
        text,
        "could not connect: connection refused (https://127.0.0.1:1/closed)"
    );
}

#[tokio::test]
async fn given_a_refused_certificate_then_the_text_is_the_reason_and_the_endpoint() {
    let server = TlsServer::start(
        Pki::other_ca().acceptor(&[b"h2"]),
        service(|_| async { Ok(Response::new(ScriptBody::new(vec![Step::Data(b"x")]))) }),
    )
    .await;
    let client = trusting_with(|knobs| knobs.connect_timeout = Duration::from_secs(5));
    let error = client
        .send(post(&server.endpoint("/v1"), "x"), &NoAuthorizer)
        .await
        .unwrap_err();
    let SendError::Tls { text } = error else {
        panic!("{error:?}");
    };
    let port = server.addr.port();
    let endpoint = format!(" (https://127.0.0.1:{port}/v1)");
    let reason = text.strip_suffix(&endpoint).unwrap_or_default();
    assert!(reason.starts_with("the server certificate "), "{text}");
    assert!(!reason.contains(" ("), "{text}");
}

#[tokio::test]
async fn given_a_connection_that_never_answers_then_the_connect_timeout_names_the_endpoint_and_the_limit()
 {
    let server = RawH2::start(|_| Policy {
        tls_fault: TlsFault::Hang,
        ..Policy::healthy()
    })
    .await;
    let client = trusting_with(|knobs| knobs.connect_timeout = CONNECT);
    let SendError::ConnectTimeout { text } = send_error(&client, &server).await else {
        panic!("a connect timeout");
    };
    let port = server.addr.port();
    assert_eq!(
        text,
        format!("no connection to https://127.0.0.1:{port}/x within 500ms")
    );
}

#[tokio::test]
async fn given_a_connection_that_carries_no_request_in_three_attempts_then_the_text_names_no_endpoint()
 {
    let server = RawH2::start(|_| Policy {
        answer: Answer::Never,
        ..Policy::announcing(Announce::Nothing)
    })
    .await;
    let client = trusting_with(|knobs| knobs.connect_timeout = CONNECT);
    let SendError::Connect { text } = send_error(&client, &server).await else {
        panic!("a connect failure");
    };
    assert_eq!(
        text,
        "no connection could carry the request; none of it was written"
    );
}

#[tokio::test]
async fn given_no_headers_in_time_then_the_headers_timeout_text_names_no_endpoint() {
    let server = RawH2::start(|_| unanswered()).await;
    let client = trusting_with(|knobs| knobs.headers_timeout = SHORT);
    let SendError::HeadersTimeout { text } = send_error(&client, &server).await else {
        panic!("a headers timeout");
    };
    assert_eq!(text, "no response headers within 200ms");
}

#[tokio::test]
async fn given_no_headers_within_the_total_timeout_then_the_total_timeout_text_names_no_endpoint() {
    let server = RawH2::start(|_| unanswered()).await;
    let client = trusting_with(|knobs| knobs.total_timeout = Some(SHORT));
    let SendError::TotalTimeout { text } = send_error(&client, &server).await else {
        panic!("a total timeout");
    };
    assert_eq!(
        text,
        "no response headers within the total timeout of 200ms"
    );
}

#[tokio::test]
async fn given_a_connection_that_ends_after_the_request_then_the_interrupted_text_names_no_endpoint()
 {
    let server = RawH2::start(|_| Policy {
        ends_after: Some((Duration::from_millis(500), Ending::Eof)),
        ..unanswered()
    })
    .await;
    let client = trusting_with(|_| {});
    let SendError::Interrupted { text } = send_error(&client, &server).await else {
        panic!("an interruption");
    };
    assert_names_no_endpoint(&text, &server);
}

async fn stalled_body_error(change: impl FnOnce(&mut Knobs)) -> BodyError {
    let server = TlsServer::start(
        Pki::trusted().acceptor(&[b"h2"]),
        service(|_| async { Ok(Response::new(ScriptBody::new(vec![Step::Stall]))) }),
    )
    .await;
    let client = trusting_with(change);
    let mut response = client
        .send(post(&server.endpoint("/"), "x"), &NoAuthorizer)
        .await
        .expect("headers");
    response.body_mut().collect(10).await.unwrap_err()
}

#[tokio::test]
async fn given_a_stalled_body_then_the_idle_timeout_text_names_no_endpoint() {
    let error = stalled_body_error(|knobs| knobs.idle_timeout = SHORT).await;
    let BodyError::IdleTimeout { text } = error else {
        panic!("{error:?}");
    };
    assert_eq!(text, "no data arrived within the idle timeout of 200ms");
}

#[tokio::test]
async fn given_a_stalled_body_then_the_total_timeout_text_names_no_endpoint() {
    let error = stalled_body_error(|knobs| knobs.total_timeout = Some(TOTAL)).await;
    let BodyError::TotalTimeout { text } = error else {
        panic!("{error:?}");
    };
    assert_eq!(
        text,
        "the response did not end within the total timeout of 500ms"
    );
}
