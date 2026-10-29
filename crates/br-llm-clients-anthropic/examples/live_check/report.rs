use br_llm_clients_core::{BoxFuture, CallOutcome, CallReport, ClientError, StreamSink};
use br_llm_messages::{ContentBlock, DeltaOp, ModelResponse, ResponseDelta};

use crate::thread::Failure;

#[derive(Default)]
pub(crate) struct QuietSink {
    pub(crate) deltas: usize,
}

impl StreamSink for QuietSink {
    fn send<'a>(&'a mut self, _delta: ResponseDelta) -> BoxFuture<'a, ()> {
        self.deltas = self.deltas.saturating_add(1);
        Box::pin(async {})
    }
}

#[derive(Default)]
pub(crate) struct PrintingSink {
    pub(crate) text_parts: usize,
}

impl StreamSink for PrintingSink {
    fn send<'a>(&'a mut self, delta: ResponseDelta) -> BoxFuture<'a, ()> {
        if let DeltaOp::TextAppended { text, .. } = delta.op() {
            self.text_parts = self.text_parts.saturating_add(1);
            crate::out::part(text);
        }
        Box::pin(async {})
    }
}

pub(crate) fn answered(outcome: CallOutcome) -> Result<ModelResponse, Failure> {
    let (report, result) = outcome.into_parts();
    print_report(&report);
    let response = result?;
    print_blocks(&response)?;
    say!("  ending: {:?}", response.ending());
    print_usage(&report);
    Ok(response)
}

pub(crate) fn print_report(report: &CallReport) {
    say!(
        "  request id: {}, answered model: {}, duration: {:?}, first event after: {:?}",
        report.request_id().unwrap_or("none"),
        report.model().map_or("none", |model| model.as_str()),
        report.total_duration(),
        report.time_to_first_event()
    );
}

fn print_blocks(response: &ModelResponse) -> Result<(), Failure> {
    for block in response.blocks() {
        match block {
            ContentBlock::Text(text) => say!("  text: {}", text.text().as_str()),
            ContentBlock::Thinking(_) => say!("  thinking block"),
            ContentBlock::RedactedThinking(_) => say!("  redacted thinking block"),
            ContentBlock::RuntimeToolCall(call) => say!(
                "  tool call: {} {}",
                call.name().as_str(),
                call.arguments().to_json_string()?
            ),
            ContentBlock::StructuredContent(value) => {
                say!("  structured: {}", value.value().to_json_string()?);
            }
            other => say!("  block of kind {:?}", other.kind()),
        }
    }
    Ok(())
}

fn print_usage(report: &CallReport) {
    let Some(usage) = report.usage() else {
        say!("  usage: not reported");
        return;
    };
    say!(
        "  usage: input {}, output {}, cache read {}, cache creation {}, thinking {}",
        count(usage.input_tokens()),
        count(usage.output_tokens()),
        count(usage.cache_read_input_tokens()),
        count(usage.cache_creation_input_tokens()),
        count(usage.thinking_tokens())
    );
}

fn count(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_owned(), |count| count.to_string())
}

pub(crate) fn error_kind(error: &ClientError) -> String {
    let name = match error {
        ClientError::ModelNotFound => "ModelNotFound",
        ClientError::ModelNotAccessible => "ModelNotAccessible",
        ClientError::Authentication => "Authentication",
        ClientError::PermissionDenied => "PermissionDenied",
        ClientError::ContextTooLong { .. } => "ContextTooLong",
        ClientError::ContentBlocked => "ContentBlocked",
        ClientError::ReplyUnusable { .. } => "ReplyUnusable",
        ClientError::InvalidMessages { .. } => "InvalidMessages",
        ClientError::QuotaExceeded => "QuotaExceeded",
        ClientError::RateLimited { .. } => "RateLimited",
        ClientError::Unavailable { .. } => "Unavailable",
        ClientError::StreamTruncated => "StreamTruncated",
        ClientError::Transport => "Transport",
        ClientError::ReasoningNotSupported { .. } => "ReasoningNotSupported",
        ClientError::StructuredOutputNotSupported => "StructuredOutputNotSupported",
        ClientError::StructuredOutputWithToolsNotSupported => {
            "StructuredOutputWithToolsNotSupported"
        }
        ClientError::ToolsNotSupported => "ToolsNotSupported",
        ClientError::CacheNotSupported => "CacheNotSupported",
        ClientError::ImageNotSupported => "ImageNotSupported",
        ClientError::ImageMediaTypeNotSupported { .. } => "ImageMediaTypeNotSupported",
        ClientError::LimitExceeded { .. } => "LimitExceeded",
        ClientError::MaxOutputTokensTooLarge { .. } => "MaxOutputTokensTooLarge",
        ClientError::ToolSchemaRejected { .. } => "ToolSchemaRejected",
        ClientError::OutputShapeRejected { .. } => "OutputShapeRejected",
        ClientError::MalformedResponse { .. } => "MalformedResponse",
        ClientError::ProviderRejected { status, code, .. } => {
            return format!("ProviderRejected (status {status:?}, code {code:?})");
        }
        ClientError::ProviderFailed { status, code, .. } => {
            return format!("ProviderFailed (status {status:?}, code {code:?})");
        }
        _ => "another error",
    };
    name.to_owned()
}
