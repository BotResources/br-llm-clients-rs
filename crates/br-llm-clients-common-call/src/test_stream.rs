use std::sync::Arc;

use br_llm_clients_core::{CallOutcome, ClientError, ClientRequest};
use br_llm_messages::{BlockStart, DeltaOp, ModelResponse, ResponseDelta, ResponseMetadataUpdate};

use crate::call_record::CallRecord;
use crate::call_spec::CallSpec;
use crate::reader::{Flow, Reader};
use crate::secrets_cache::SecretsCache;
use crate::test_decoder::{DecoderLog, EndScript};
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::spec_for;
use crate::test_ids::number_of;
use crate::test_server::{TestServer, closed_endpoint, http};
use crate::test_sink::RecordingSink;

pub(crate) fn frame(lines: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in lines {
        bytes.extend_from_slice(format!("data: {line}\n").as_bytes());
    }
    bytes.push(b'\n');
    bytes
}

pub(crate) fn body(frames: &[&[&str]]) -> Vec<u8> {
    frames.iter().flat_map(|lines| frame(lines)).collect()
}

pub(crate) fn dialect_ending(end: EndScript) -> Arc<ScriptedDialect> {
    Arc::new(ScriptedDialect {
        end,
        ..ScriptedDialect::default()
    })
}

pub(crate) struct Driven {
    pub(crate) result: Result<ModelResponse, ClientError>,
    pub(crate) sink: RecordingSink,
    pub(crate) record: CallRecord,
}

pub(crate) async fn drive(
    dialect: &Arc<ScriptedDialect>,
    request: &ClientRequest,
    chunks: &[Vec<u8>],
) -> Driven {
    let spec = spec_for(dialect, closed_endpoint());
    drive_spec(&spec, &dialect.log, request, chunks).await
}

pub(crate) async fn drive_spec(
    spec: &CallSpec,
    log: &Arc<DecoderLog>,
    request: &ClientRequest,
    chunks: &[Vec<u8>],
) -> Driven {
    let mut sink = RecordingSink::watching(log);
    let mut record = CallRecord::start();
    let result = pump(spec, request, &mut sink, &mut record, chunks).await;
    Driven {
        result,
        sink,
        record,
    }
}

async fn pump(
    spec: &CallSpec,
    request: &ClientRequest,
    sink: &mut RecordingSink,
    record: &mut CallRecord,
    chunks: &[Vec<u8>],
) -> Result<ModelResponse, ClientError> {
    let secrets = SecretsCache::default();
    let mut reader = Reader::new(spec, &secrets, request, sink, record);
    for chunk in chunks {
        if let Flow::Complete = reader.chunk(chunk).await? {
            return reader.conclude().await;
        }
    }
    reader.end_of_body().await?;
    reader.conclude().await
}

pub(crate) async fn drive_one(
    dialect: &Arc<ScriptedDialect>,
    request: &ClientRequest,
    bytes: Vec<u8>,
) -> Driven {
    drive(dialect, request, &[bytes]).await
}

pub(crate) struct Called {
    pub(crate) outcome: CallOutcome,
    pub(crate) sink: RecordingSink,
    pub(crate) requests: usize,
}

pub(crate) async fn call_with(
    dialect: &Arc<ScriptedDialect>,
    request: &ClientRequest,
    bytes: Vec<u8>,
) -> Called {
    let server = TestServer::start(move |_| http("200 OK", &[], &bytes)).await;
    let spec = spec_for(dialect, server.endpoint("/v1/test"));
    let mut sink = RecordingSink::watching(&dialect.log);
    let outcome = crate::run::run(&spec, request, &mut sink).await;
    Called {
        outcome,
        sink,
        requests: server.count(),
    }
}

pub(crate) fn kinds(deltas: &[ResponseDelta]) -> Vec<String> {
    deltas.iter().map(|delta| describe(delta.op())).collect()
}

fn describe(op: &DeltaOp) -> String {
    match op {
        DeltaOp::ResponseStarted { metadata, .. } => match metadata.model() {
            Some(model) => format!("started {}", model.as_str()),
            None => "started".to_owned(),
        },
        DeltaOp::BlockStarted { block, start, .. } => {
            format!("start {} {}", number_of(*block), describe_start(start))
        }
        DeltaOp::TextAppended { block, text, .. } => {
            format!("text {} {text:?}", number_of(*block))
        }
        DeltaOp::ThinkingAppended { block, text, .. } => {
            format!("think {} {text:?}", number_of(*block))
        }
        DeltaOp::JsonAppended {
            block, fragment, ..
        } => format!("json {} {fragment:?}", number_of(*block)),
        DeltaOp::ReplayFragmentAppended {
            block,
            format,
            fragment,
            ..
        } => format!(
            "replay {} {} {fragment:?}",
            number_of(*block),
            format.as_str()
        ),
        DeltaOp::BlockFinalized { block, .. } => format!("final {}", number_of(*block)),
        DeltaOp::ResponseMetadataUpdated { update, .. } => describe_update(update),
        DeltaOp::ResponseCompleted { ending, .. } => format!("done {ending:?}"),
        other => format!("{other:?}"),
    }
}

fn describe_start(start: &BlockStart) -> String {
    match start {
        BlockStart::Text { .. } => "text".to_owned(),
        BlockStart::Thinking { .. } => "thinking".to_owned(),
        BlockStart::RedactedThinking { .. } => "redacted".to_owned(),
        BlockStart::StructuredContent { .. } => "structured".to_owned(),
        BlockStart::RuntimeToolCall {
            name, external_id, ..
        } => format!(
            "call {} {}",
            name.as_ref().map_or("-", |name| name.as_str()),
            external_id.as_ref().map_or("-", |id| id.as_str())
        ),
        other => format!("{other:?}"),
    }
}

fn describe_update(update: &ResponseMetadataUpdate) -> String {
    let mut parts = Vec::new();
    if let Some(model) = update.model() {
        parts.push(format!("model={}", model.as_str()));
    }
    if let Some(usage) = update.usage() {
        parts.push(format!(
            "usage={:?}/{:?}",
            usage.input_tokens(),
            usage.output_tokens()
        ));
    }
    if let Some(id) = update.provider_response_id() {
        parts.push(format!("rid={}", id.as_str()));
    }
    format!("meta {}", parts.join(" "))
}
