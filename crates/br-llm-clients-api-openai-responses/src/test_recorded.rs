use br_llm_clients_common_call::{Dialect, Payload, PayloadDecoder, ProviderEvent, Secret, run};
use br_llm_clients_common_sse::Decoder;
use br_llm_clients_core::{ClientRequest, OutputMode, ParameterKind, ReasoningLevel, ToolSpec};
use br_llm_messages::ModelResponse;
use serde_json::Value;

use crate::OpenAiResponses;
use crate::test_recordings::Recording;
use crate::test_requests::{dialect, hello, parameter, request_with, shape, tool, user_text};
use crate::test_run::{Called, RecordingSink, call_with_wire, spec_with};
use crate::test_server::{TestServer, Wire, http};

pub(crate) fn payloads_of(chunks: &[&[u8]]) -> Vec<Payload> {
    let mut decoder = Decoder::default();
    let mut frames = Vec::new();
    for chunk in chunks {
        decoder.feed(chunk, &mut frames).expect("sse bytes");
    }
    decoder.finish().expect("sse end");
    frames
        .into_iter()
        .map(|frame| {
            let (event, data) = frame.into_parts();
            Payload::new(event, data)
        })
        .collect()
}

pub(crate) fn decode_all(
    dialect: &OpenAiResponses,
    mode: &OutputMode,
    payloads: Vec<Payload>,
) -> Vec<ProviderEvent> {
    let mut decoder: Box<dyn PayloadDecoder> = dialect.decoder(mode);
    payloads
        .into_iter()
        .flat_map(|payload| decoder.payload(payload).expect("a readable payload"))
        .collect()
}

pub(crate) fn decode_recording(recording: &Recording, mode: &OutputMode) -> Vec<ProviderEvent> {
    let payloads = payloads_of(&[recording.body.as_bytes()]);
    decode_all(&dialect(ReasoningLevel::Off), mode, payloads)
}

pub(crate) fn recorded_json(recording: &Recording) -> Vec<Value> {
    payloads_of(&[recording.body.as_bytes()])
        .iter()
        .map(|payload| serde_json::from_str(payload.data()).expect("a JSON payload"))
        .collect()
}

pub(crate) fn recorded_event(recording: &Recording, kind: &str, nth: usize) -> Value {
    recorded_json(recording)
        .into_iter()
        .filter(|event| event["type"] == kind)
        .nth(nth)
        .expect("the recorded event")
}

pub(crate) fn recorded_wire(recording: &Recording) -> Wire {
    http(
        recording.status_line,
        &[
            ("content-type", recording.content_type),
            ("x-request-id", recording.request_id),
        ],
        recording.body.as_bytes(),
    )
}

pub(crate) fn weather_request() -> ClientRequest {
    request_with(
        None,
        vec![user_text("What is the weather in Paris?")],
        vec![weather_tool()],
        OutputMode::Plain,
    )
}

fn weather_tool() -> ToolSpec {
    tool(
        "get_weather",
        "",
        vec![parameter("city", "", true, ParameterKind::String)],
    )
}

pub(crate) fn answer_mode() -> OutputMode {
    OutputMode::Structured(shape(
        "answer",
        None,
        vec![parameter("steps", "", true, ParameterKind::String)],
    ))
}

pub(crate) fn answer_request() -> ClientRequest {
    request_with(None, vec![user_text("go")], Vec::new(), answer_mode())
}

pub(crate) async fn run_recording(
    recording: &Recording,
    level: ReasoningLevel,
    request: &ClientRequest,
) -> Called {
    let wire = recorded_wire(recording);
    call_with_wire(dialect(level), request, move |_| wire.clone()).await
}

pub(crate) async fn run_plain(recording: &Recording) -> Called {
    run_recording(recording, ReasoningLevel::Off, &hello()).await
}

pub(crate) async fn run_split(
    recording: &Recording,
    level: ReasoningLevel,
    request: &ClientRequest,
    at: usize,
) -> Called {
    let wire = recorded_wire(recording).split_body_at(at);
    call_with_wire(dialect(level), request, move |_| wire.clone()).await
}

pub(crate) async fn run_with_secrets(
    recording: &Recording,
    secrets: &[&str],
) -> br_llm_clients_core::CallOutcome {
    let wire = recorded_wire(recording);
    let server = TestServer::start(move |_| wire.clone()).await;
    let secrets = secrets
        .iter()
        .map(|value| Secret::new(*value).expect("secret"))
        .collect();
    let spec = spec_with(
        dialect(ReasoningLevel::Off),
        server.endpoint("/v1/responses"),
    )
    .with_secrets(secrets);
    let mut sink = RecordingSink::default();
    run(&spec, &hello(), &mut sink).await
}

pub(crate) fn reply_of(called: Called) -> ModelResponse {
    called.reply()
}
