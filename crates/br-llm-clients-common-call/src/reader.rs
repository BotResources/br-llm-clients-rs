use br_llm_clients_common_sse::{Decoder, Frame};
use br_llm_clients_core::{ClientError, ClientRequest, StreamSink};
use br_llm_messages::Step;

use crate::call_record::CallRecord;
use crate::call_spec::CallSpec;
use crate::classify::classify_stream_error;
use crate::conclude::conclude;
use crate::emitter::Emitter;
use crate::error_info::ErrorInfo;
use crate::failure_map::{decode_failure, payload_failure};
use crate::normalizer::Normalizer;
use crate::payload::Payload;
use crate::payload_decoder::{End, PayloadDecoder};
use crate::provider_event::ProviderEvent;

pub(crate) enum Flow {
    More,
    Complete,
}

pub(crate) struct Reader<'a> {
    spec: &'a CallSpec,
    request: &'a ClientRequest,
    record: &'a mut CallRecord,
    sse: Decoder,
    payloads: Box<dyn PayloadDecoder>,
    normalizer: Normalizer,
    emitter: Emitter<'a>,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(
        spec: &'a CallSpec,
        request: &'a ClientRequest,
        sink: &'a mut dyn StreamSink,
        record: &'a mut CallRecord,
    ) -> Self {
        Self {
            spec,
            request,
            record,
            sse: Decoder::default(),
            payloads: spec.dialect.decoder(request.output()),
            normalizer: Normalizer::new(request),
            emitter: Emitter::new(sink),
        }
    }

    pub(crate) async fn chunk(&mut self, bytes: &[u8]) -> Result<Flow, ClientError> {
        let mut frames = Vec::new();
        let fed = self.sse.feed(bytes, &mut frames);
        if let Flow::Complete = self.frames(frames).await? {
            return Ok(Flow::Complete);
        }
        fed.map_err(|error| decode_failure(&error))?;
        Ok(Flow::More)
    }

    pub(crate) async fn end_of_body(&mut self) -> Result<(), ClientError> {
        self.sse.finish().map_err(|error| decode_failure(&error))?;
        match self.payloads.end() {
            Err(error) => Err(payload_failure(&error)),
            Ok(End::MarkerMissing) => Err(ClientError::StreamTruncated),
            Ok(End::Events(events)) => self.list(events).await.map(drop),
        }
    }

    pub(crate) async fn conclude(mut self) -> Result<Step, ClientError> {
        let conclusion = conclude(
            &mut self.normalizer,
            self.emitter.log(),
            self.request,
            self.record
                .answered_model
                .as_ref()
                .unwrap_or(&self.spec.model),
        )?;
        for event in conclusion.tail {
            self.emitter.emit(event, self.record).await?;
        }
        Ok(conclusion.step)
    }

    async fn frames(&mut self, frames: Vec<Frame>) -> Result<Flow, ClientError> {
        for frame in frames {
            let events = self
                .payloads
                .payload(Payload::from_frame(frame))
                .map_err(|error| payload_failure(&error))?;
            if let Flow::Complete = self.list(events).await? {
                return Ok(Flow::Complete);
            }
        }
        Ok(Flow::More)
    }

    async fn list(&mut self, events: Vec<ProviderEvent>) -> Result<Flow, ClientError> {
        let mut events = events.into_iter();
        while let Some(event) = events.next() {
            match event {
                ProviderEvent::Complete => return Ok(Flow::Complete),
                ProviderEvent::Error(info) => return Err(self.classify(info)),
                other => {
                    if let Err(failure) = self.apply(other).await {
                        return Err(self.first_error(events).unwrap_or(failure));
                    }
                }
            }
        }
        Ok(Flow::More)
    }

    fn first_error(&self, rest: impl Iterator<Item = ProviderEvent>) -> Option<ClientError> {
        for event in rest {
            match event {
                ProviderEvent::Complete => return None,
                ProviderEvent::Error(info) => return Some(self.classify(info)),
                _ => {}
            }
        }
        None
    }

    fn classify(&self, info: ErrorInfo) -> ClientError {
        classify_stream_error(&*self.spec.dialect, &self.spec.secrets_in_effect(), info)
    }

    async fn apply(&mut self, event: ProviderEvent) -> Result<(), ClientError> {
        let produced = self.normalizer.accept(event)?;
        self.record.usage = self.normalizer.usage().cloned();
        self.record.model = self.normalizer.model().cloned();
        for event in produced {
            self.emitter.emit(event, self.record).await?;
        }
        Ok(())
    }
}
