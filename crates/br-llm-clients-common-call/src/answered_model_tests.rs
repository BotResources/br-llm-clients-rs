use std::sync::Arc;

use br_llm_clients_common_http::{Headers, Secret};
use br_llm_clients_core::{CallOutcome, ClientError, ClientRequest, OutputMode, RenderedHistory};

use crate::dialect::Dialect;
use crate::encoded_request::EncodedRequest;
use crate::error_info::ErrorInfo;
use crate::payload_decoder::PayloadDecoder;
use crate::secret_log::SecretLog;
use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{run_call, spec_for, user_request};
use crate::test_server::{TestServer, http};
use crate::test_stream::{body, dialect_ending};

const GOOD: &[&str] = &["start a text", "text a hi", "end a", "stop finished"];

fn answering(header: &'static str) -> Arc<ScriptedDialect> {
    Arc::new(ScriptedDialect {
        answered: Box::new(move |headers| headers.get_text(header).map(str::to_owned)),
        end: EndScript::Events(Vec::new()),
        ..ScriptedDialect::default()
    })
}

async fn call(
    dialect: Arc<dyn Dialect>,
    status: &'static str,
    headers: &'static [(&'static str, &'static str)],
    lines: &'static [&'static str],
    secret: Option<&str>,
) -> CallOutcome {
    let payload = body(&[lines]);
    let server = TestServer::start(move |_| http(status, headers, &payload)).await;
    let mut spec = crate::call_spec::CallSpec::new(
        dialect,
        server.endpoint("/v1/test"),
        Arc::new(br_llm_clients_common_http::NoAuthorizer),
        crate::test_fixtures::http_client(),
    );
    if let Some(secret) = secret {
        spec = spec.with_secrets(vec![Secret::new(secret).expect("secret")]);
    }
    run_call(&spec, &user_request()).await
}

fn model_of(outcome: CallOutcome) -> (Option<String>, Option<String>) {
    let (report, result) = outcome.into_parts();
    let step = result.ok().and_then(|response| response.model().cloned());
    (
        step.map(|model| model.as_str().to_owned()),
        report.model().map(|model| model.as_str().to_owned()),
    )
}

#[tokio::test]
async fn given_a_valid_answered_model_when_the_reply_has_no_model_event_then_response_and_report_hold_it()
 {
    let outcome = call(
        answering("x-model"),
        "200 OK",
        &[("x-model", "gw-model")],
        GOOD,
        None,
    )
    .await;
    assert_eq!(
        model_of(outcome),
        (Some("gw-model".to_owned()), Some("gw-model".to_owned()))
    );
}

#[tokio::test]
async fn given_a_valid_answered_model_when_the_call_is_refused_then_the_report_holds_it() {
    let outcome = call(
        answering("x-model"),
        "500 Bad",
        &[("x-model", "gw-model")],
        &[],
        None,
    )
    .await;
    assert_eq!(model_of(outcome), (None, Some("gw-model".to_owned())));
}

#[tokio::test]
async fn given_a_valid_answered_model_when_the_stream_fails_midway_then_the_report_holds_it() {
    let lines: &[&str] = &["start a text", "text a hi"];
    let outcome = call(
        answering("x-model"),
        "200 OK",
        &[("x-model", "gw-model")],
        lines,
        None,
    )
    .await;
    let (report, result) = outcome.into_parts();
    assert_eq!(result, Err(ClientError::StreamTruncated));
    assert_eq!(report.model().map(|model| model.as_str()), Some("gw-model"));
}

#[tokio::test]
async fn given_an_answered_model_and_a_model_event_when_the_reply_ends_then_the_event_replaces_it()
{
    let lines: &[&str] = &[
        "model event-model",
        "start a text",
        "text a hi",
        "end a",
        "stop finished",
    ];
    let outcome = call(
        answering("x-model"),
        "200 OK",
        &[("x-model", "gw-model")],
        lines,
        None,
    )
    .await;
    assert_eq!(
        model_of(outcome),
        (
            Some("event-model".to_owned()),
            Some("event-model".to_owned())
        )
    );
}

#[tokio::test]
async fn given_an_answered_model_that_is_not_valid_when_run_then_it_is_absent_and_the_call_goes_on()
{
    let dialect = Arc::new(ScriptedDialect {
        answered: Box::new(|_| Some(String::new())),
        end: EndScript::Events(Vec::new()),
        ..ScriptedDialect::default()
    });
    let outcome = call(dialect, "200 OK", &[], GOOD, None).await;
    assert_eq!(model_of(outcome), (None, None));
}

#[tokio::test]
async fn given_an_answered_model_with_a_secret_in_it_when_run_then_it_is_absent_and_the_call_goes_on()
 {
    let headers: &[(&str, &str)] = &[("x-model", "proxy-sk-live-9-model")];
    let outcome = call(
        answering("x-model"),
        "200 OK",
        headers,
        GOOD,
        Some("sk-live-9"),
    )
    .await;
    assert_eq!(model_of(outcome), (None, None));
    let refused = call(
        answering("x-model"),
        "500 Bad",
        headers,
        &[],
        Some("sk-live-9"),
    )
    .await;
    assert_eq!(model_of(refused), (None, None));
}

#[tokio::test]
async fn given_an_answered_model_equal_to_a_secret_recorded_in_the_log_when_run_then_it_is_absent()
{
    let dialect = answering("x-model");
    let payload = body(&[GOOD]);
    let server =
        TestServer::start(move |_| http("200 OK", &[("x-model", "tok-logged")], &payload)).await;
    let log = SecretLog::new();
    log.record(&Secret::new("tok-logged").expect("secret"));
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secret_log(log);
    let outcome = run_call(&spec, &user_request()).await;
    assert_eq!(model_of(outcome), (None, None));
}

struct Bare(Arc<ScriptedDialect>);

impl Dialect for Bare {
    fn encode(
        &self,
        request: &ClientRequest,
        history: &RenderedHistory,
    ) -> Result<EncodedRequest, ClientError> {
        self.0.encode(request, history)
    }

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder> {
        self.0.decoder(output)
    }

    fn error_code(&self, body: &str) -> Option<String> {
        self.0.error_code(body)
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        self.0.refine_error(info)
    }

    fn request_id(&self, headers: &Headers) -> Option<String> {
        self.0.request_id(headers)
    }
}

#[tokio::test]
async fn given_a_dialect_that_does_not_implement_the_method_when_run_then_the_models_are_as_in_0_1()
{
    let bare = Arc::new(Bare(dialect_ending(EndScript::Events(Vec::new()))));
    let outcome = call(bare, "200 OK", &[("x-model", "gw-model")], GOOD, None).await;
    assert_eq!(model_of(outcome), (None, None));
}
