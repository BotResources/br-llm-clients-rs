use std::sync::Arc;

use serde_json::json;

use crate::test_support::{feed, finish_chunk, hello, settings};
use crate::{
    ChatCompletionsDialect, ChatCompletionsOptions, ClientError, Dialect, DialectHooks,
    EncodedRequest, End, ErrorInfo, Layered, OutputMode, Payload, PayloadDecoder, PayloadError,
    ProviderEvent, ReasoningLevel, ReasoningTable, RenderedHistory, ResponseEnding,
};

struct Remapped {
    inner: Box<dyn PayloadDecoder>,
}

impl PayloadDecoder for Remapped {
    fn payload(&mut self, payload: Payload) -> Result<Vec<ProviderEvent>, PayloadError> {
        let events = self.inner.payload(payload)?;
        Ok(events
            .into_iter()
            .map(|event| match event {
                ProviderEvent::Stop(ResponseEnding::Unclassified { .. }) => {
                    ProviderEvent::Stop(ResponseEnding::finished())
                }
                other => other,
            })
            .collect())
    }

    fn end(&mut self) -> Result<End, PayloadError> {
        self.inner.end()
    }
}

struct HostHooks;

impl DialectHooks for HostHooks {
    fn after_encode(&self, encoded: EncodedRequest) -> Result<EncodedRequest, ClientError> {
        Ok(encoded.with_header("x-host", "1"))
    }

    fn wrap_decoder(&self, inner: Box<dyn PayloadDecoder>) -> Box<dyn PayloadDecoder> {
        Box::new(Remapped { inner })
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        (info.code() == Some("host_out_of_credit")).then_some(ClientError::QuotaExceeded)
    }
}

fn wrapped() -> Layered<HostHooks> {
    let options = ChatCompletionsOptions::new(ReasoningTable::openai_effort());
    let dialect =
        ChatCompletionsDialect::new(settings(ReasoningLevel::Off), options).expect("dialect");
    let inner: Arc<dyn Dialect> = Arc::new(dialect);
    Layered::new(inner, HostHooks)
}

#[test]
fn given_a_layered_dialect_built_from_the_re_exports_when_used_then_the_hooks_change_body_events_and_codes()
 {
    let layered = wrapped();
    let request = hello();
    let history: RenderedHistory =
        br_llm_clients_core::render_history(&request).expect("rendering");
    let encoded = layered.encode(&request, &history).expect("encoded");
    assert!(encoded.headers().iter().any(|(name, _)| name == "x-host"));
    let mut decoder = layered.decoder(&OutputMode::Plain);
    let events = feed(decoder.as_mut(), &finish_chunk("host_paused")).expect("chunk");
    assert!(events.contains(&ProviderEvent::Stop(ResponseEnding::finished())));
    let info = ErrorInfo::new("body")
        .with_status(402)
        .with_code("host_out_of_credit");
    assert_eq!(
        layered.refine_error(&info),
        Some(ClientError::QuotaExceeded)
    );
    let quota = ErrorInfo::new("body")
        .with_status(429)
        .with_code("insufficient_quota");
    assert_eq!(
        layered.refine_error(&quota),
        Some(ClientError::QuotaExceeded)
    );
    assert_eq!(
        layered.error_code(&json!({"error": {"code": "c"}}).to_string()),
        Some("c".to_owned())
    );
}
