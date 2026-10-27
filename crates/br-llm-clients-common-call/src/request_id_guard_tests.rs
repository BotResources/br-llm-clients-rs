use std::sync::Arc;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::CallReport;

use crate::call_spec::CallSpec;
use crate::secret_log::SecretLog;
use crate::test_authorizer::RecordingAuthorizer;
use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{http_client, run_call, spec_for, user_request};
use crate::test_server::{TestServer, http};
use crate::test_stream::{body, dialect_ending};

const LIST: &str = "key-in-list";
const TOKEN: &str = "tok-recorded-in-send";

#[derive(Clone, Copy)]
enum Kind {
    Reply,
    Refusal,
}

async fn report(kind: Kind, request_id: &'static str) -> CallReport {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let reply = body(&[&["start a text", "text a hi", "end a", "stop finished"]]);
    let server = TestServer::start(move |_| match kind {
        Kind::Reply => http("200 OK", &[("x-request-id", request_id)], &reply),
        Kind::Refusal => http("500 Bad", &[("x-request-id", request_id)], b"{}"),
    })
    .await;
    let log = SecretLog::new();
    let spec = recording_spec(&dialect, &server, &log)
        .with_secrets(vec![Secret::new(LIST).expect("secret")]);
    run_call(&spec, &user_request()).await.into_parts().0
}

fn recording_spec(
    dialect: &Arc<ScriptedDialect>,
    server: &TestServer,
    log: &SecretLog,
) -> CallSpec {
    CallSpec::new(
        dialect.clone(),
        server.endpoint("/v1/test"),
        Arc::new(RecordingAuthorizer {
            log: log.clone(),
            token: TOKEN,
        }),
        http_client(),
    )
    .with_secret_log(log.clone())
}

#[tokio::test]
async fn given_a_request_id_that_is_a_secret_when_the_outcome_is_ready_then_the_report_has_none() {
    for kind in [Kind::Reply, Kind::Refusal] {
        assert_eq!(report(kind, LIST).await.request_id(), None);
        assert_eq!(report(kind, TOKEN).await.request_id(), None);
    }
}

#[tokio::test]
async fn given_a_request_id_with_a_secret_inside_when_the_outcome_is_ready_then_the_report_has_none()
 {
    for kind in [Kind::Reply, Kind::Refusal] {
        assert_eq!(report(kind, "req-key-in-list-1").await.request_id(), None);
        assert_eq!(
            report(kind, "req-tok-recorded-in-send-1")
                .await
                .request_id(),
            None
        );
    }
}

#[tokio::test]
async fn given_a_clean_request_id_when_the_outcome_is_ready_then_it_is_kept_as_received() {
    for kind in [Kind::Reply, Kind::Refusal] {
        assert_eq!(
            report(kind, "req-Clean_42").await.request_id(),
            Some("req-Clean_42")
        );
    }
}

#[tokio::test]
async fn given_a_secret_recorded_in_a_spec_without_the_id_when_the_id_is_clean_then_it_is_kept() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("500 Bad", &[("x-request-id", "req-1")], b"{}")).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"))
        .with_secrets(vec![Secret::new(LIST).expect("secret")]);
    let report = run_call(&spec, &user_request()).await.into_parts().0;
    assert_eq!(report.request_id(), Some("req-1"));
}
