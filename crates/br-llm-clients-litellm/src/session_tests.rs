use br_llm_clients_core::{Client, ClientError};

use crate::call_options::LiteLlmCallOptions;
use crate::session_id::SessionId;
use crate::test_server::{Reply, TestServer};
use crate::test_support::{KEY, NullSink, answer_stream, client, hello};

const SESSION_HEADER: &str = "x-litellm-session-id";
const SESSION: &str = "0190a0b1-0000-7000-8000-000000000001";

fn ok_reply(headers: Vec<(&'static str, &'static str)>) -> Reply {
    let mut headers = headers;
    headers.push(("content-type", "text/event-stream"));
    Reply {
        status: "200 OK",
        headers,
        body: answer_stream(),
    }
}

fn session_options(value: &str) -> LiteLlmCallOptions {
    LiteLlmCallOptions::new().with_session_id(SessionId::new(value).unwrap())
}

#[tokio::test]
async fn given_a_session_id_when_called_with_it_then_the_session_header_holds_it() {
    let server = TestServer::start(ok_reply(Vec::new())).await;
    let client = client(server.base(), Vec::new());
    let options = session_options(SESSION);
    let outcome = client.call_with(&hello(), &options, &mut NullSink).await;
    assert!(outcome.outcome().result().is_ok(), "{outcome:?}");
    assert_eq!(server.header_values(SESSION_HEADER), vec![SESSION]);
}

#[tokio::test]
async fn given_no_session_id_when_called_with_options_then_no_session_header_is_sent() {
    let server = TestServer::start(ok_reply(Vec::new())).await;
    let client = client(server.base(), Vec::new());
    let outcome = client
        .call_with(&hello(), &LiteLlmCallOptions::new(), &mut NullSink)
        .await;
    assert!(outcome.outcome().result().is_ok(), "{outcome:?}");
    assert!(server.header_values(SESSION_HEADER).is_empty());
}

#[tokio::test]
async fn given_the_client_trait_when_called_then_no_session_header_is_sent() {
    let server = TestServer::start(ok_reply(Vec::new())).await;
    let client = client(server.base(), Vec::new());
    let outcome = Client::call(&client, &hello(), &mut NullSink).await;
    assert!(outcome.result().is_ok(), "{outcome:?}");
    assert!(server.header_values(SESSION_HEADER).is_empty());
}

#[tokio::test]
async fn given_a_fixed_session_header_and_a_session_id_when_called_then_the_session_id_replaces_it()
{
    let server = TestServer::start(ok_reply(Vec::new())).await;
    let fixed = vec![("X-LiteLLM-Session-Id".to_owned(), "fixed".to_owned())];
    let client = client(server.base(), fixed);
    let outcome = client
        .call_with(&hello(), &session_options(SESSION), &mut NullSink)
        .await;
    assert!(outcome.outcome().result().is_ok(), "{outcome:?}");
    assert_eq!(server.header_values(SESSION_HEADER), vec![SESSION]);
}

#[tokio::test]
async fn given_a_fixed_session_header_and_no_session_id_when_called_then_the_fixed_value_is_sent() {
    let server = TestServer::start(ok_reply(Vec::new())).await;
    let fixed = vec![(SESSION_HEADER.to_owned(), "fixed".to_owned())];
    let client = client(server.base(), fixed);
    let outcome = client
        .call_with(&hello(), &LiteLlmCallOptions::new(), &mut NullSink)
        .await;
    assert!(outcome.outcome().result().is_ok(), "{outcome:?}");
    assert_eq!(server.header_values(SESSION_HEADER), vec!["fixed"]);
}

#[tokio::test]
async fn given_a_served_by_header_when_called_then_the_deployment_id_and_the_request_id_are_given()
{
    let server = TestServer::start(ok_reply(vec![
        ("x-litellm-model-id", "deployment-a"),
        ("x-litellm-call-id", "call-1"),
    ]))
    .await;
    let client = client(server.base(), Vec::new());
    let outcome = client
        .call_with(&hello(), &session_options(SESSION), &mut NullSink)
        .await;
    assert!(outcome.outcome().result().is_ok(), "{outcome:?}");
    assert_eq!(outcome.deployment_id(), Some("deployment-a"));
    assert_eq!(outcome.outcome().report().request_id(), Some("call-1"));
}

#[tokio::test]
async fn given_no_served_by_header_when_called_then_no_deployment_id_is_given() {
    let server = TestServer::start(ok_reply(vec![("x-litellm-call-id", "call-1")])).await;
    let client = client(server.base(), Vec::new());
    let outcome = client
        .call_with(&hello(), &session_options(SESSION), &mut NullSink)
        .await;
    assert!(outcome.outcome().result().is_ok(), "{outcome:?}");
    assert_eq!(outcome.deployment_id(), None);
    assert_eq!(outcome.outcome().report().request_id(), Some("call-1"));
}

#[tokio::test]
async fn given_an_empty_served_by_header_when_called_then_no_deployment_id_is_given() {
    let server = TestServer::start(ok_reply(vec![("x-litellm-model-id", "")])).await;
    let client = client(server.base(), Vec::new());
    let outcome = client
        .call_with(&hello(), &LiteLlmCallOptions::new(), &mut NullSink)
        .await;
    assert_eq!(outcome.deployment_id(), None);
}

#[tokio::test]
async fn given_a_served_by_header_on_an_error_reply_when_called_then_the_deployment_id_is_given() {
    let server = TestServer::start(Reply {
        status: "503 Service Unavailable",
        headers: vec![
            ("x-litellm-model-id", "deployment-b"),
            ("content-type", "application/json"),
        ],
        body: br#"{"error":{"message":"busy","type":"None","code":"503"}}"#.to_vec(),
    })
    .await;
    let client = client(server.base(), Vec::new());
    let outcome = client
        .call_with(&hello(), &session_options(SESSION), &mut NullSink)
        .await;
    assert!(
        matches!(
            outcome.outcome().result(),
            Err(ClientError::Unavailable { .. })
        ),
        "{outcome:?}"
    );
    assert_eq!(outcome.deployment_id(), Some("deployment-b"));
}

#[tokio::test]
async fn given_a_served_by_header_that_holds_the_key_when_called_then_no_deployment_id_is_given() {
    let leaked: &'static str = Box::leak(format!("deployment-{KEY}").into_boxed_str());
    let server = TestServer::start(ok_reply(vec![("x-litellm-model-id", leaked)])).await;
    let client = client(server.base(), Vec::new());
    let outcome = client
        .call_with(&hello(), &LiteLlmCallOptions::new(), &mut NullSink)
        .await;
    assert!(outcome.outcome().result().is_ok(), "{outcome:?}");
    assert_eq!(outcome.deployment_id(), None);
}

#[tokio::test]
async fn given_two_calls_at_once_on_one_client_when_served_by_two_deployments_then_each_gets_its_own()
 {
    let first_deployment =
        TestServer::start(ok_reply(vec![("x-litellm-model-id", "deployment-a")])).await;
    let second_deployment =
        TestServer::start(ok_reply(vec![("x-litellm-model-id", "deployment-b")])).await;
    let to_first = client(first_deployment.base(), Vec::new());
    let to_second = client(second_deployment.base(), Vec::new());
    let first = session_options("session-a");
    let second = session_options("session-b");
    let request = hello();
    let (mut sink_a, mut sink_b) = (NullSink, NullSink);
    let (a, b) = tokio::join!(
        to_first.call_with(&request, &first, &mut sink_a),
        to_second.call_with(&request, &second, &mut sink_b),
    );
    assert_eq!(a.deployment_id(), Some("deployment-a"));
    assert_eq!(b.deployment_id(), Some("deployment-b"));
    assert_eq!(
        first_deployment.header_values(SESSION_HEADER),
        vec!["session-a"]
    );
    assert_eq!(
        second_deployment.header_values(SESSION_HEADER),
        vec!["session-b"]
    );
}

#[tokio::test]
async fn given_one_client_when_called_in_two_sessions_then_each_call_sends_its_own_session_id() {
    let server = TestServer::start(ok_reply(Vec::new())).await;
    let client = client(server.base(), Vec::new());
    for session in ["session-a", "session-b"] {
        let outcome = client
            .call_with(&hello(), &session_options(session), &mut NullSink)
            .await;
        assert!(outcome.outcome().result().is_ok(), "{outcome:?}");
    }
    assert_eq!(
        server.values_per_request(SESSION_HEADER),
        vec![vec!["session-a"], vec!["session-b"]]
    );
}
