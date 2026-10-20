use std::sync::{Arc, Mutex};

use br_llm_clients_core::{ClientError, OutputMode};

use crate::dialect::Dialect;
use crate::dialect_hooks::DialectHooks;
use crate::encoded_request::EncodedRequest;
use crate::error_info::ErrorInfo;
use crate::layered::Layered;
use crate::payload::Payload;
use crate::payload_decoder::{End, PayloadDecoder};
use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::user_request;

struct Tracing {
    inner: Arc<ScriptedDialect>,
    trace: Mutex<Vec<String>>,
    check_error: Option<ClientError>,
    after_error: Option<ClientError>,
}

impl Tracing {
    fn new(inner: &Arc<ScriptedDialect>) -> Self {
        Self {
            inner: Arc::clone(inner),
            trace: Mutex::new(Vec::new()),
            check_error: None,
            after_error: None,
        }
    }

    fn note(&self, step: &str) {
        let line = format!("{step} after {} inner encodes", self.inner.encode_calls());
        self.trace.lock().expect("trace").push(line);
    }

    fn trace(&self) -> Vec<String> {
        self.trace.lock().expect("trace").clone()
    }
}

impl DialectHooks for Arc<Tracing> {
    fn check(&self, _request: &br_llm_clients_core::ClientRequest) -> Result<(), ClientError> {
        self.note("check");
        self.check_error.clone().map_or(Ok(()), Err)
    }

    fn after_encode(&self, encoded: EncodedRequest) -> Result<EncodedRequest, ClientError> {
        self.note("after_encode");
        match &self.after_error {
            Some(error) => Err(error.clone()),
            None => Ok(encoded.with_header("x-after", "1")),
        }
    }
}

fn layered(inner: &Arc<ScriptedDialect>, hooks: &Arc<Tracing>) -> Layered<Arc<Tracing>> {
    Layered::new(inner.clone(), Arc::clone(hooks))
}

#[test]
fn given_the_public_types_when_compiled_then_layered_is_send_and_sync() {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<Layered<Arc<Tracing>>>();
    is_send_sync::<Layered<Plain>>();
}

#[test]
fn given_a_request_when_layered_encodes_then_check_comes_before_the_inner_encode_and_after_encode_last()
 {
    let inner = Arc::new(ScriptedDialect::default());
    let hooks = Arc::new(Tracing::new(&inner));
    let encoded = layered(&inner, &hooks)
        .encode(&user_request())
        .expect("encoded");
    assert_eq!(
        hooks.trace(),
        vec![
            "check after 0 inner encodes",
            "after_encode after 1 inner encodes"
        ]
    );
    assert!(
        encoded
            .headers()
            .contains(&("x-after".to_owned(), "1".to_owned()))
    );
    assert_eq!(encoded.body(), inner.body.as_slice());
}

#[test]
fn given_a_check_error_when_layered_encodes_then_the_inner_dialect_is_not_asked() {
    let inner = Arc::new(ScriptedDialect::default());
    let mut tracing = Tracing::new(&inner);
    tracing.check_error = Some(ClientError::StructuredOutputWithToolsNotSupported);
    let hooks = Arc::new(tracing);
    let error = layered(&inner, &hooks).encode(&user_request());
    assert_eq!(
        error,
        Err(ClientError::StructuredOutputWithToolsNotSupported)
    );
    assert_eq!(inner.encode_calls(), 0);
    assert_eq!(hooks.trace(), vec!["check after 0 inner encodes"]);
}

#[test]
fn given_an_inner_error_when_layered_encodes_then_it_is_returned_unchanged_and_after_encode_is_not_called()
 {
    let failure = ClientError::ToolSchemaRejected {
        tool: None,
        reason: "no".to_owned(),
    };
    let inner = Arc::new(ScriptedDialect {
        encode_error: Some(failure.clone()),
        ..ScriptedDialect::default()
    });
    let hooks = Arc::new(Tracing::new(&inner));
    let error = layered(&inner, &hooks).encode(&user_request());
    assert_eq!(error, Err(failure));
    assert_eq!(hooks.trace(), vec!["check after 0 inner encodes"]);
}

#[test]
fn given_an_after_encode_error_when_layered_encodes_then_that_error_is_the_answer() {
    let inner = Arc::new(ScriptedDialect::default());
    let mut tracing = Tracing::new(&inner);
    tracing.after_error = Some(ClientError::StructuredOutputWithToolsNotSupported);
    let hooks = Arc::new(tracing);
    let error = layered(&inner, &hooks).encode(&user_request());
    assert_eq!(
        error,
        Err(ClientError::StructuredOutputWithToolsNotSupported)
    );
    assert_eq!(inner.encode_calls(), 1);
}

struct Refining(Option<ClientError>);

impl DialectHooks for Refining {
    fn refine_error(&self, _info: &ErrorInfo) -> Option<ClientError> {
        self.0.clone()
    }
}

fn refining_inner(answer: Option<ClientError>) -> Arc<ScriptedDialect> {
    Arc::new(ScriptedDialect {
        refine: Box::new(move |_| answer.clone()),
        ..ScriptedDialect::default()
    })
}

#[test]
fn given_a_hook_that_refines_when_layered_refines_then_the_hook_wins_and_the_inner_is_not_asked() {
    let inner = refining_inner(Some(ClientError::PermissionDenied));
    let layered = Layered::new(inner.clone(), Refining(Some(ClientError::Authentication)));
    let answer = layered.refine_error(&ErrorInfo::new("x"));
    assert_eq!(answer, Some(ClientError::Authentication));
    assert!(inner.seen_infos().is_empty());
}

#[test]
fn given_a_hook_that_gives_nothing_when_layered_refines_then_the_inner_answer_is_the_answer() {
    let inner = refining_inner(Some(ClientError::PermissionDenied));
    let layered = Layered::new(inner.clone(), Refining(None));
    let info = ErrorInfo::new("x").with_status(400);
    assert_eq!(
        layered.refine_error(&info),
        Some(ClientError::PermissionDenied)
    );
    assert_eq!(inner.seen_infos(), vec![info.clone()]);
    let empty = Layered::new(refining_inner(None), Refining(None));
    assert_eq!(empty.refine_error(&info), None);
}

struct Coding {
    seen: Mutex<Vec<(String, Option<String>)>>,
    answer: Option<Option<String>>,
}

impl DialectHooks for Coding {
    fn error_code(&self, body: &str, inner: Option<String>) -> Option<String> {
        self.seen
            .lock()
            .expect("seen")
            .push((body.to_owned(), inner.clone()));
        self.answer.clone().unwrap_or(inner)
    }
}

#[test]
fn given_a_hook_on_the_code_when_layered_reads_it_then_the_hook_gets_the_inner_answer_and_decides()
{
    let coding = |answer| Coding {
        seen: Mutex::new(Vec::new()),
        answer,
    };
    let inner = Arc::new(ScriptedDialect::default());
    let body = r#"{"code":"inner-c"}"#;
    let replaced = Layered::new(inner.clone(), coding(Some(Some("hook-c".to_owned()))));
    assert_eq!(replaced.error_code(body), Some("hook-c".to_owned()));
    let unchanged = Layered::new(inner.clone(), coding(None));
    assert_eq!(unchanged.error_code(body), Some("inner-c".to_owned()));
    let added = Layered::new(inner.clone(), coding(Some(Some("added".to_owned()))));
    assert_eq!(added.error_code("{}"), Some("added".to_owned()));
    let removed = Layered::new(inner, coding(Some(None)));
    assert_eq!(removed.error_code(body), None);
}

#[test]
fn given_a_hook_on_the_code_when_layered_reads_it_then_the_hook_saw_the_body_and_the_inner_answer()
{
    let hooks = Arc::new(Coding {
        seen: Mutex::new(Vec::new()),
        answer: None,
    });
    struct Shared(Arc<Coding>);
    impl DialectHooks for Shared {
        fn error_code(&self, body: &str, inner: Option<String>) -> Option<String> {
            self.0.error_code(body, inner)
        }
    }
    let layered = Layered::new(Arc::new(ScriptedDialect::default()), Shared(hooks.clone()));
    layered.error_code(r#"{"code":"c1"}"#);
    layered.error_code("{}");
    assert_eq!(
        hooks.seen.lock().expect("seen").clone(),
        vec![
            (r#"{"code":"c1"}"#.to_owned(), Some("c1".to_owned())),
            ("{}".to_owned(), None)
        ]
    );
}

struct Plain;

impl DialectHooks for Plain {}

fn drain(decoder: &mut Box<dyn PayloadDecoder>) -> (Vec<String>, End) {
    let mut seen = Vec::new();
    for data in ["start a text\ntext a hi", "end a\nstop end_turn"] {
        let events = decoder.payload(Payload::new(None, data)).expect("events");
        seen.extend(events.iter().map(|event| format!("{event:?}")));
    }
    (seen, decoder.end().expect("end"))
}

#[test]
fn given_hooks_with_every_default_when_layered_answers_then_every_answer_is_the_inner_answer() {
    let inner = Arc::new(ScriptedDialect {
        refine: Box::new(|_| Some(ClientError::Authentication)),
        end: EndScript::Events(vec!["usage 1 2".to_owned()]),
        ..ScriptedDialect::default()
    });
    let layered = Layered::new(inner.clone(), Plain);
    let request = user_request();
    assert_eq!(layered.encode(&request), inner.encode(&request));
    let body = r#"{"code":"c9"}"#;
    assert_eq!(layered.error_code(body), inner.error_code(body));
    assert_eq!(layered.error_code("{}"), inner.error_code("{}"));
    let info = ErrorInfo::new("x").with_status(401);
    assert_eq!(layered.refine_error(&info), inner.refine_error(&info));
    let created = inner.log.created();
    let mut wrapped = layered.decoder(&OutputMode::Plain);
    let mut direct = inner.decoder(&OutputMode::Plain);
    assert_eq!(inner.log.created(), created + 2);
    assert_eq!(drain(&mut wrapped), drain(&mut direct));
    assert_eq!(
        inner.seen_modes(),
        vec![OutputMode::Plain, OutputMode::Plain]
    );
}
