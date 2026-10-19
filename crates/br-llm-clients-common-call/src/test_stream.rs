use std::sync::Arc;

use br_llm_clients_core::{CallOutcome, ClientError, ClientRequest};
use br_llm_messages::{Step, StreamEvent};

use crate::call_record::CallRecord;
use crate::call_spec::CallSpec;
use crate::reader::{Flow, Reader};
use crate::test_decoder::{DecoderLog, EndScript};
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::spec_for;
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
    pub(crate) result: Result<Step, ClientError>,
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
) -> Result<Step, ClientError> {
    let mut reader = Reader::new(spec, request, sink, record);
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

pub(crate) fn kinds(events: &[StreamEvent]) -> Vec<String> {
    events.iter().map(describe).collect()
}

fn describe(event: &StreamEvent) -> String {
    match event {
        StreamEvent::BlockStart { index, kind } => format!("start {index} {kind:?}"),
        StreamEvent::ToolCallStart { index, id, name } => format!("call {index} {id} {name}"),
        StreamEvent::TextDelta { index, text } => format!("text {index} {text:?}"),
        StreamEvent::ThinkingDelta { index, text } => format!("think {index} {text:?}"),
        StreamEvent::SignatureDelta { index, signature } => format!("sig {index} {signature:?}"),
        StreamEvent::RedactedThinkingData { index, data } => format!("redact {index} {data:?}"),
        StreamEvent::StructuredDelta {
            index,
            json_fragment,
        } => format!("struct {index} {json_fragment:?}"),
        StreamEvent::ToolCallArgumentsDelta {
            index,
            json_fragment,
        } => format!("args {index} {json_fragment:?}"),
        StreamEvent::BlockEnd { index } => format!("end {index}"),
        StreamEvent::Finish { stop_reason, usage } => {
            format!("finish {stop_reason} {}", usage.is_some())
        }
        other => format!("{other:?}"),
    }
}
