use std::sync::Arc;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::ClientError;

use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{failure_of, run_call, spec_for, user_request};
use crate::test_server::{TestServer, http};
use crate::test_stream::{body, dialect_ending};

const KEY: &str = "sk/a\"b+c";
const SERDE_FORM: &str = "sk/a\\\"b+c";
const PHP_FORM: &str = "sk\\/a\\\"b+c";
const DOTNET_FORM: &str = "sk/a\\u0022b\\u002Bc";

fn key() -> Secret {
    Secret::new(KEY).expect("secret")
}

fn refusal(form: &str) -> Vec<u8> {
    format!("{{\"error\":{{\"message\":\"Incorrect API key provided: {form}\"}}}}").into_bytes()
}

#[tokio::test]
async fn given_a_refusal_that_repeats_the_key_json_escaped_when_run_then_every_text_is_cleaned() {
    for form in [SERDE_FORM, PHP_FORM, DOTNET_FORM] {
        let dialect = Arc::new(ScriptedDialect::default());
        let reply = refusal(form);
        let server = TestServer::start(move |_| http("400 Bad", &[], &reply)).await;
        let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![key()]);
        let error = failure_of(&spec, &user_request()).await;
        let expected = String::from_utf8(refusal("[redacted]")).expect("utf8");
        assert_eq!(dialect.seen_bodies(), vec![expected.clone()]);
        assert_eq!(
            error,
            ClientError::ProviderRejected {
                status: Some(400),
                code: None,
                body: expected
            }
        );
    }
}

#[tokio::test]
async fn given_a_cut_refusal_that_ends_inside_the_escaped_key_when_run_then_no_part_of_it_is_kept()
{
    for kept in 1..DOTNET_FORM.len() {
        let dialect = Arc::new(ScriptedDialect::default());
        let mut reply = vec![b'x'; 65_536 - kept];
        reply.extend_from_slice(DOTNET_FORM.as_bytes());
        reply.extend_from_slice(&[b'y'; 100]);
        let server = TestServer::start(move |_| http("500 Bad", &[], &reply)).await;
        let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![key()]);
        let error = failure_of(&spec, &user_request()).await;
        let expected = "x".repeat(65_536 - kept);
        assert_eq!(dialect.seen_bodies(), vec![expected.clone()], "{kept}");
        assert_eq!(
            error,
            ClientError::ProviderFailed {
                status: Some(500),
                code: None,
                body: expected
            }
        );
    }
}

#[tokio::test]
async fn given_an_error_event_that_repeats_the_key_json_escaped_when_run_then_its_texts_are_cleaned()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let event = format!("error 400 {PHP_FORM} - bad key {SERDE_FORM}");
    let server = TestServer::start(move |_| http("200 OK", &[], &body(&[&[event.as_str()]]))).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![key()]);
    let error = failure_of(&spec, &user_request()).await;
    let info = &dialect.seen_infos()[0];
    assert_eq!(
        (info.code(), info.text()),
        (Some("[redacted]"), "bad key [redacted]")
    );
    assert_eq!(
        error,
        ClientError::ProviderRejected {
            status: Some(400),
            code: Some("[redacted]".to_owned()),
            body: "bad key [redacted]".to_owned()
        }
    );
}

#[tokio::test]
async fn given_a_request_id_that_holds_the_escaped_key_when_the_outcome_is_ready_then_it_is_absent()
{
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let id = format!("req-{SERDE_FORM}");
    let server = TestServer::start(move |_| http("500 Bad", &[("x-request-id", &id)], b"{}")).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![key()]);
    let (report, _) = run_call(&spec, &user_request()).await.into_parts();
    assert_eq!(report.request_id(), None);
}
