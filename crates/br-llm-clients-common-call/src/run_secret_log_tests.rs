use std::sync::Arc;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::{CallOutcome, ClientError};

use crate::call_spec::CallSpec;
use crate::secret_log::SecretLog;
use crate::test_authorizer::RecordingAuthorizer;
use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{http_client, run_call, spec_for, user_request};
use crate::test_server::{TestServer, http};
use crate::test_stream::{body, dialect_ending};

const TOKEN: &str = "tok-fetched-in-send";

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

fn rejected(outcome: CallOutcome) -> (Option<String>, String) {
    match outcome.into_parts().1 {
        Err(ClientError::ProviderRejected { code, body, .. }) => (code, body),
        other => panic!("not rejected: {other:?}"),
    }
}

#[tokio::test]
async fn given_a_secret_recorded_while_send_runs_when_the_refusal_repeats_it_then_every_text_is_cleaned()
 {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| {
        let body = format!(r#"{{"code":"{TOKEN}","msg":"bad {TOKEN}"}}"#);
        http("400 Bad", &[], body.as_bytes())
    })
    .await;
    let log = SecretLog::new();
    let spec = recording_spec(&dialect, &server, &log);
    let (code, body) = rejected(run_call(&spec, &user_request()).await);
    assert_eq!(code.as_deref(), Some("[redacted]"));
    assert_eq!(
        body,
        r#"{"code":"[redacted]","msg":"bad [redacted]"}"#.to_owned()
    );
    assert!(!dialect.seen_bodies()[0].contains(TOKEN));
    assert!(!dialect.seen_infos()[0].text().contains(TOKEN));
    assert_eq!(dialect.seen_infos()[0].code(), Some("[redacted]"));
}

#[tokio::test]
async fn given_a_secret_recorded_while_send_runs_when_an_error_event_repeats_it_then_the_texts_are_cleaned()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let line = format!("error 400 {TOKEN} - bad {TOKEN}");
    let server = TestServer::start(move |_| http("200 OK", &[], &body(&[&[line.as_str()]]))).await;
    let log = SecretLog::new();
    let spec = recording_spec(&dialect, &server, &log);
    let (code, text) = rejected(run_call(&spec, &user_request()).await);
    assert_eq!(code.as_deref(), Some("[redacted]"));
    assert_eq!(text, "bad [redacted]");
    let info = &dialect.seen_infos()[0];
    assert_eq!(
        (info.code(), info.text()),
        (Some("[redacted]"), "bad [redacted]")
    );
}

#[tokio::test]
async fn given_a_secret_recorded_only_after_the_outcome_is_ready_when_read_then_nothing_changes() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("400 Bad", &[], TOKEN.as_bytes())).await;
    let log = SecretLog::new();
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secret_log(log.clone());
    let outcome = run_call(&spec, &user_request()).await;
    log.record(&Secret::new(TOKEN).expect("token"));
    let (_, text) = rejected(outcome);
    assert_eq!(text, TOKEN);
    let (_, text) = rejected(run_call(&spec, &user_request()).await);
    assert_eq!(text, "[redacted]");
}

#[tokio::test]
async fn given_a_secret_recorded_in_a_clone_of_the_log_when_run_then_the_spec_sees_it() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("400 Bad", &[], b"x tok-clone y")).await;
    let log = SecretLog::new();
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secret_log(log.clone());
    log.clone()
        .record(&Secret::new("tok-clone").expect("token"));
    let (_, text) = rejected(run_call(&spec, &user_request()).await);
    assert_eq!(text, "x [redacted] y");
}

#[tokio::test]
async fn given_a_new_log_set_on_the_spec_when_run_then_the_earlier_log_is_replaced() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("400 Bad", &[], b"x tok-old y")).await;
    let old = SecretLog::new();
    old.record(&Secret::new("tok-old").expect("token"));
    let spec = spec_for(&dialect, server.endpoint("/v1/test"))
        .with_secret_log(old)
        .with_secret_log(SecretLog::new());
    let (_, text) = rejected(run_call(&spec, &user_request()).await);
    assert_eq!(text, "x tok-old y");
}

#[tokio::test]
async fn given_a_spec_made_with_new_when_run_then_its_log_is_empty_and_the_list_still_works() {
    let dialect = Arc::new(ScriptedDialect::default());
    let server = TestServer::start(|_| http("400 Bad", &[], b"x tok-list y tok-open z")).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"))
        .with_secrets(vec![Secret::new("tok-list").expect("token")]);
    let (_, text) = rejected(run_call(&spec, &user_request()).await);
    assert_eq!(text, "x [redacted] y tok-open z");
}

#[tokio::test]
async fn given_a_secret_in_a_text_made_by_the_dialect_when_only_the_log_holds_it_then_the_text_is_cleaned()
 {
    let dialect = Arc::new(ScriptedDialect {
        refine: Box::new(|_| {
            Some(ClientError::OutputShapeRejected {
                reason: format!("bad {TOKEN}"),
            })
        }),
        ..ScriptedDialect::default()
    });
    let server = TestServer::start(|_| http("400 Bad", &[], b"{}")).await;
    let log = SecretLog::new();
    let spec = recording_spec(&dialect, &server, &log);
    let error = run_call(&spec, &user_request()).await.into_parts().1;
    assert_eq!(
        error,
        Err(ClientError::OutputShapeRejected {
            reason: "bad [redacted]".to_owned()
        })
    );
}

struct RecordingSink {
    log: SecretLog,
    token: &'static str,
}

impl br_llm_clients_core::StreamSink for RecordingSink {
    fn send<'a>(
        &'a mut self,
        _delta: br_llm_messages::ResponseDelta,
    ) -> br_llm_clients_core::BoxFuture<'a, ()> {
        Box::pin(async move {
            self.log.record(&Secret::new(self.token).expect("token"));
        })
    }
}

#[tokio::test]
async fn given_a_secret_recorded_after_the_headers_when_the_outcome_is_ready_then_every_text_and_the_request_id_are_cleaned()
 {
    const LATE: &str = "tok-recorded-mid-stream";
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let line = format!("error 400 {LATE} - bad {LATE}");
    let id = format!("req-{LATE}");
    let server = TestServer::start(move |_| {
        let lines = ["start a text", "text a hi", line.as_str()];
        http("200 OK", &[("x-request-id", id.as_str())], &body(&[&lines]))
    })
    .await;
    let log = SecretLog::new();
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secret_log(log.clone());
    let mut sink = RecordingSink { log, token: LATE };
    let outcome = crate::run::run(&spec, &user_request(), &mut sink).await;
    let (report, result) = outcome.into_parts();
    assert_eq!(report.request_id(), None);
    assert_eq!(
        result,
        Err(ClientError::ProviderRejected {
            status: Some(400),
            code: Some("[redacted]".to_owned()),
            body: "bad [redacted]".to_owned()
        })
    );
    let info = &dialect.seen_infos()[0];
    assert_eq!(
        (info.code(), info.text()),
        (Some("[redacted]"), "bad [redacted]")
    );
}

#[tokio::test]
async fn given_a_logged_secret_across_the_cut_of_an_error_event_text_when_run_then_the_replacement_comes_before_the_cut()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let text = format!("{}{TOKEN}", "a".repeat(65_531));
    let line = format!("error 400 - - {text}");
    let server = TestServer::start(move |_| http("200 OK", &[], &body(&[&[line.as_str()]]))).await;
    let log = SecretLog::new();
    log.record(&Secret::new(TOKEN).expect("token"));
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secret_log(log);
    let (_, text) = rejected(run_call(&spec, &user_request()).await);
    let expected = format!("{}[reda", "a".repeat(65_531));
    assert_eq!(text, expected);
    assert_eq!(dialect.seen_infos()[0].text(), expected);
}
