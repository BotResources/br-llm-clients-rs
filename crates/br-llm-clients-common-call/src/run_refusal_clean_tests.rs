use std::sync::Arc;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::ClientError;

use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{failure_of, spec_for, user_request};
use crate::test_server::{TestServer, http};
use crate::test_stream::{body, dialect_ending};

fn secret(value: &str) -> Secret {
    Secret::new(value).expect("secret")
}

#[tokio::test]
async fn given_a_cut_refusal_that_the_replacement_grows_when_run_then_the_dialect_gets_64_kib() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("500 Bad", &[], &vec![b'k'; 70_000])).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![secret("k")]);
    let error = failure_of(&spec, &user_request()).await;
    let expected = "[redacted]".repeat(6_554)[..65_536].to_owned();
    assert_eq!(dialect.seen_bodies(), vec![expected.clone()]);
    assert_eq!(dialect.seen_infos()[0].text(), expected);
    let ClientError::ProviderFailed { body, .. } = error else {
        panic!("not failed");
    };
    assert_eq!(body.len(), 65_536);
    assert!(!body.contains('k'));
}

#[tokio::test]
async fn given_a_refusal_whose_secret_is_in_the_placeholder_when_run_then_the_outcome_cleans_the_placeholder_again()
 {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("400 Bad", &[], b"red")).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![secret("red")]);
    let error = failure_of(&spec, &user_request()).await;
    assert_eq!(dialect.seen_bodies(), vec!["[redacted]".to_owned()]);
    assert_eq!(dialect.seen_infos()[0].text(), "[redacted]");
    assert_eq!(
        error,
        ClientError::ProviderRejected {
            status: Some(400),
            code: None,
            body: "[[redacted]acted]".to_owned()
        }
    );
}

#[tokio::test]
async fn given_an_error_event_whose_secret_is_in_the_placeholder_when_run_then_the_outcome_cleans_the_placeholder_again()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let server =
        TestServer::start(|_| http("200 OK", &[], &body(&[&["error 400 red - red"]]))).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![secret("red")]);
    let error = failure_of(&spec, &user_request()).await;
    let info = &dialect.seen_infos()[0];
    assert_eq!(
        (info.code(), info.text()),
        (Some("[redacted]"), "[redacted]")
    );
    assert_eq!(
        error,
        ClientError::ProviderRejected {
            status: Some(400),
            code: Some("[[redacted]acted]".to_owned()),
            body: "[[redacted]acted]".to_owned()
        }
    );
}
