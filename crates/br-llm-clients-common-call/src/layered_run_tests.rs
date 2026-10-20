use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::Headers;
use br_llm_clients_core::{ClientError, ClientRequest};

use crate::dialect_hooks::DialectHooks;
use crate::encoded_request::EncodedRequest;
use crate::layered::Layered;
use crate::payload::{Payload, PayloadError};
use crate::payload_decoder::{End, PayloadDecoder};
use crate::provider_event::ProviderEvent;
use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{http_client, model, run_call, user_request};
use crate::test_server::{TestServer, http};
use crate::test_stream::body;

const GOOD: &[&str] = &["start a text", "text a hi", "end a", "stop end_turn"];

fn inner() -> Arc<ScriptedDialect> {
    Arc::new(ScriptedDialect {
        answered: Box::new(|headers| headers.get_text("x-model").map(str::to_owned)),
        end: EndScript::Events(Vec::new()),
        ..ScriptedDialect::default()
    })
}

async fn serve_good(headers: &'static [(&'static str, &'static str)]) -> TestServer {
    let payload = body(&[GOOD]);
    TestServer::start(move |_| http("200 OK", headers, &payload)).await
}

fn spec<H: DialectHooks + 'static>(hooks: H, server: &TestServer) -> crate::call_spec::CallSpec {
    crate::call_spec::CallSpec::new(
        Arc::new(Layered::new(inner(), hooks)),
        server.endpoint("/v1/test"),
        Arc::new(br_llm_clients_common_http::NoAuthorizer),
        http_client(),
        model(),
    )
}

#[derive(Default)]
struct Ids {
    seen_id: Mutex<Vec<Option<String>>>,
    seen_model: Mutex<Vec<Option<String>>>,
}

struct IdHooks(Arc<Ids>);

impl DialectHooks for IdHooks {
    fn request_id(&self, headers: &Headers, inner: Option<String>) -> Option<String> {
        self.0.seen_id.lock().expect("seen").push(inner.clone());
        inner
            .map(|id| format!("{id}-hooked"))
            .or_else(|| headers.get_text("x-alt-id").map(str::to_owned))
    }

    fn answered_model(&self, _headers: &Headers, inner: Option<String>) -> Option<String> {
        self.0.seen_model.lock().expect("seen").push(inner.clone());
        inner.map(|model| format!("{model}-hooked"))
    }
}

#[tokio::test]
async fn given_a_hook_on_the_request_id_when_run_then_it_gets_the_inner_answer_and_decides() {
    let server = serve_good(&[("x-request-id", "req-9"), ("x-model", "gw")]).await;
    let ids = Arc::new(Ids::default());
    let outcome = run_call(&spec(IdHooks(ids.clone()), &server), &user_request()).await;
    let (report, result) = outcome.into_parts();
    assert_eq!(report.request_id(), Some("req-9-hooked"));
    assert_eq!(
        *ids.seen_id.lock().expect("seen"),
        vec![Some("req-9".to_owned())]
    );
    assert_eq!(
        *ids.seen_model.lock().expect("seen"),
        vec![Some("gw".to_owned())]
    );
    assert_eq!(
        report.model().map(|model| model.as_str()),
        Some("gw-hooked")
    );
    assert_eq!(
        result.expect("a step").model().map(|model| model.as_str()),
        Some("gw-hooked")
    );
}

#[tokio::test]
async fn given_a_hook_on_the_request_id_when_the_inner_gives_none_then_the_hook_can_read_another_header()
 {
    let server = serve_good(&[("x-alt-id", "alt-3")]).await;
    let ids = Arc::new(Ids::default());
    let report = run_call(&spec(IdHooks(ids.clone()), &server), &user_request())
        .await
        .into_parts()
        .0;
    assert_eq!(report.request_id(), Some("alt-3"));
    assert_eq!(*ids.seen_id.lock().expect("seen"), vec![None]);
    assert_eq!(report.model(), None);
}

struct Refusing;

impl DialectHooks for Refusing {
    fn check(&self, _request: &ClientRequest) -> Result<(), ClientError> {
        Err(ClientError::StructuredOutputWithToolsNotSupported)
    }
}

#[tokio::test]
async fn given_a_check_error_when_run_then_nothing_is_sent_and_the_error_is_the_outcome() {
    let server = serve_good(&[]).await;
    let error = run_call(&spec(Refusing, &server), &user_request())
        .await
        .into_parts()
        .1;
    assert_eq!(
        error,
        Err(ClientError::StructuredOutputWithToolsNotSupported)
    );
    assert_eq!(server.count(), 0);
}

struct Rewriting;

impl DialectHooks for Rewriting {
    fn after_encode(&self, _encoded: EncodedRequest) -> Result<EncodedRequest, ClientError> {
        Ok(EncodedRequest::new(b"{\"rewritten\":true}".to_vec()).with_header("x-after", "yes"))
    }
}

#[tokio::test]
async fn given_an_after_encode_hook_when_run_then_the_server_gets_the_rewritten_request() {
    let server = serve_good(&[]).await;
    run_call(&spec(Rewriting, &server), &user_request()).await;
    let received = server.requests();
    assert_eq!(received[0].body, b"{\"rewritten\":true}");
    assert_eq!(received[0].header_values("x-after"), vec!["yes"]);
    assert!(received[0].header_values("content-type").is_empty());
}

struct Counting {
    payloads: Arc<AtomicUsize>,
    inner: Box<dyn PayloadDecoder>,
}

impl PayloadDecoder for Counting {
    fn payload(&mut self, payload: Payload) -> Result<Vec<ProviderEvent>, PayloadError> {
        self.payloads.fetch_add(1, Ordering::SeqCst);
        self.inner.payload(payload)
    }

    fn end(&mut self) -> Result<End, PayloadError> {
        self.inner.end()
    }
}

struct Wrapping(Arc<AtomicUsize>);

impl DialectHooks for Wrapping {
    fn wrap_decoder(&self, inner: Box<dyn PayloadDecoder>) -> Box<dyn PayloadDecoder> {
        Box::new(Counting {
            payloads: Arc::clone(&self.0),
            inner,
        })
    }
}

#[tokio::test]
async fn given_a_decoder_wrapper_when_run_then_the_call_reads_through_it() {
    let server = serve_good(&[]).await;
    let payloads = Arc::new(AtomicUsize::new(0));
    let outcome = run_call(&spec(Wrapping(payloads.clone()), &server), &user_request()).await;
    assert!(outcome.into_parts().1.is_ok());
    assert_eq!(payloads.load(Ordering::SeqCst), 1);
}

struct Untouched;

impl DialectHooks for Untouched {}

#[tokio::test]
async fn given_hooks_with_every_default_when_run_then_the_request_id_and_the_models_are_the_inner_ones()
 {
    let server = serve_good(&[("x-request-id", "req-7"), ("x-model", "gw")]).await;
    let direct = crate::call_spec::CallSpec::new(
        inner(),
        server.endpoint("/v1/test"),
        Arc::new(br_llm_clients_common_http::NoAuthorizer),
        http_client(),
        model(),
    );
    let (direct_report, direct_step) = run_call(&direct, &user_request()).await.into_parts();
    let (layered_report, layered_step) = run_call(&spec(Untouched, &server), &user_request())
        .await
        .into_parts();
    assert_eq!(direct_report.request_id(), Some("req-7"));
    assert_eq!(
        direct_report.model().map(|model| model.as_str()),
        Some("gw")
    );
    assert_eq!(layered_report.request_id(), direct_report.request_id());
    assert_eq!(layered_report.model(), direct_report.model());
    assert_eq!(
        layered_step.expect("a step").model(),
        direct_step.expect("a step").model()
    );
}
