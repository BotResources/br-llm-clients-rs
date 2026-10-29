use br_llm_clients_core::{ClientError, ClientRequest, StreamSink};
use br_llm_messages::{ModelResponse, ResponseDelta, ResponseDraft};

use crate::call_record::CallRecord;
use crate::failure_map::message_failure;

pub(crate) struct Emitter<'a> {
    draft: ResponseDraft,
    sink: &'a mut dyn StreamSink,
}

impl<'a> Emitter<'a> {
    pub(crate) fn new(request: &ClientRequest, sink: &'a mut dyn StreamSink) -> Self {
        Self {
            draft: ResponseDraft::new(request.reply_turn(), request.reply_entry()),
            sink,
        }
    }

    pub(crate) fn draft(&self) -> &ResponseDraft {
        &self.draft
    }

    pub(crate) fn into_response(self) -> Option<ModelResponse> {
        self.draft.into_response()
    }

    pub(crate) async fn emit(
        &mut self,
        delta: ResponseDelta,
        record: &mut CallRecord,
    ) -> Result<(), ClientError> {
        self.draft
            .apply(&delta)
            .map_err(|error| message_failure(&error))?;
        if record.time_to_first_event.is_none() {
            record.time_to_first_event = Some(record.started.elapsed());
        }
        self.sink.send(delta).await;
        Ok(())
    }
}
