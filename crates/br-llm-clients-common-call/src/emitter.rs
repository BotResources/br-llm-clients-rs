use br_llm_clients_core::{ClientError, StreamSink};
use br_llm_messages::{StepDraft, StreamEvent};

use crate::call_record::CallRecord;
use crate::failure_map::message_failure;

pub(crate) struct Emitter<'a> {
    draft: StepDraft,
    log: Vec<StreamEvent>,
    sink: &'a mut dyn StreamSink,
}

impl<'a> Emitter<'a> {
    pub(crate) fn new(sink: &'a mut dyn StreamSink) -> Self {
        Self {
            draft: StepDraft::new(),
            log: Vec::new(),
            sink,
        }
    }

    pub(crate) fn log(&self) -> &[StreamEvent] {
        &self.log
    }

    pub(crate) async fn emit(
        &mut self,
        event: StreamEvent,
        record: &mut CallRecord,
    ) -> Result<(), ClientError> {
        self.draft
            .apply(event.clone())
            .map_err(|error| message_failure(&error))?;
        if record.time_to_first_event.is_none() {
            record.time_to_first_event = Some(record.started.elapsed());
        }
        self.log.push(event.clone());
        self.sink.send(event).await;
        Ok(())
    }
}
