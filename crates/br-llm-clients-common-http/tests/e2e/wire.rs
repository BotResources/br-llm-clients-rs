use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::{Endpoint, NoAuthorizer, Request};

use crate::support::raw::{RawServer, Received, read_request, write_response};
use crate::support::{client, client_with, guarded, post};

async fn capturing_server() -> (RawServer, Arc<Mutex<Vec<Received>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let server = RawServer::start(move |mut stream, _| {
        let sink = Arc::clone(&sink);
        async move {
            while let Ok(received) = read_request(&mut stream).await {
                sink.lock().expect("lock").push(received);
                if write_response(&mut stream, "200 OK", &[], b"ok")
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    })
    .await;
    (server, seen)
}

#[tokio::test]
async fn given_a_request_when_sent_then_it_is_one_post_with_the_exact_path_query_and_body() {
    let (server, seen) = capturing_server().await;
    let endpoint = server.endpoint("/v1/models/a%2Fb:gen?k=v%20w&x");
    let body: Vec<u8> = (0u8..=255).collect();
    let request = Request::new(endpoint, body.clone());
    let response = guarded(client().send(request, &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(response.status(), 200);
    let seen = seen.lock().expect("lock");
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].request_line,
        "POST /v1/models/a%2Fb:gen?k=v%20w&x HTTP/1.1"
    );
    assert_eq!(seen[0].body, body);
}

#[tokio::test]
async fn given_a_request_when_sent_then_the_library_headers_are_exactly_these() {
    let (server, seen) = capturing_server().await;
    let endpoint = server.endpoint("/p");
    let request = post(&endpoint, "{}")
        .with_header("Content-Type", "application/json")
        .expect("header");
    guarded(client().send(request, &NoAuthorizer))
        .await
        .expect("response");
    let seen = seen.lock().expect("lock");
    let mut names: Vec<String> = seen[0]
        .headers
        .iter()
        .map(|(n, _)| n.to_ascii_lowercase())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "accept-encoding",
            "content-length",
            "content-type",
            "host",
            "user-agent"
        ]
    );
    assert_eq!(
        seen[0].header("host"),
        Some(format!("127.0.0.1:{}", server.addr.port()).as_str())
    );
    assert_eq!(seen[0].header("accept-encoding"), Some("identity"));
    assert_eq!(seen[0].header("content-length"), Some("2"));
    assert_eq!(seen[0].header("content-type"), Some("application/json"));
}

#[tokio::test]
async fn given_no_user_agent_when_sent_then_a_fixed_value_names_the_library_and_its_version() {
    let (server, seen) = capturing_server().await;
    guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let agent = seen.lock().expect("lock")[0]
        .header("user-agent")
        .map(str::to_owned);
    assert_eq!(agent.as_deref(), Some("br-llm-clients-common-http/0.1.0"));
}

#[tokio::test]
async fn given_a_configured_user_agent_when_sent_then_it_is_used() {
    let (server, seen) = capturing_server().await;
    let client = client_with(|c| c.user_agent = Some("my-app/9".to_owned()));
    guarded(client.send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(
        seen.lock().expect("lock")[0].header("user-agent"),
        Some("my-app/9")
    );
}

#[tokio::test]
async fn given_a_user_agent_on_the_request_when_sent_then_it_wins_and_is_sent_once() {
    let (server, seen) = capturing_server().await;
    let client = client_with(|c| c.user_agent = Some("my-app/9".to_owned()));
    let request = post(&server.endpoint("/"), "")
        .with_header("user-agent", "per-request/1")
        .expect("header");
    guarded(client.send(request, &NoAuthorizer))
        .await
        .expect("response");
    let seen = seen.lock().expect("lock");
    assert_eq!(seen[0].header_count("user-agent"), 1);
    assert_eq!(seen[0].header("user-agent"), Some("per-request/1"));
}

#[tokio::test]
async fn given_an_empty_body_when_sent_then_the_content_length_is_zero() {
    let (server, seen) = capturing_server().await;
    guarded(client().send(post(&server.endpoint("/"), ""), &NoAuthorizer))
        .await
        .expect("response");
    let seen = seen.lock().expect("lock");
    assert_eq!(seen[0].header("content-length"), Some("0"));
    assert!(seen[0].body.is_empty());
}

#[tokio::test]
async fn given_an_endpoint_with_an_ipv6_literal_when_sent_then_the_host_header_keeps_the_brackets()
{
    let listener = tokio::net::TcpListener::bind("[::1]:0").await;
    let Ok(listener) = listener else { return };
    let port = listener.local_addr().expect("addr").port();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let received = read_request(&mut stream).await.expect("request");
        write_response(&mut stream, "200 OK", &[], b"")
            .await
            .expect("response");
        received
    });
    let endpoint = Endpoint::plain_http(&format!("http://[::1]:{port}/v6")).expect("endpoint");
    guarded(client().send(post(&endpoint, ""), &NoAuthorizer))
        .await
        .expect("response");
    let received = task.await.expect("task");
    assert_eq!(
        received.header("host"),
        Some(format!("[::1]:{port}").as_str())
    );
}

#[tokio::test]
async fn given_localhost_when_sent_then_name_resolution_and_address_fallback_work() {
    let (server, seen) = capturing_server().await;
    let endpoint = Endpoint::plain_http(&format!("http://localhost:{}/l", server.addr.port()))
        .expect("endpoint");
    guarded(client().send(post(&endpoint, "x"), &NoAuthorizer))
        .await
        .expect("response");
    assert_eq!(seen.lock().expect("lock").len(), 1);
}
