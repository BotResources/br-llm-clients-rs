use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::{ModelId, Step, StepDraft, StreamEvent};

use crate::failure_map::message_failure;
use crate::judge::judge;
use crate::normalizer::Normalizer;

pub(crate) struct Conclusion {
    pub(crate) tail: Vec<StreamEvent>,
    pub(crate) step: Step,
}

pub(crate) fn conclude(
    normalizer: &mut Normalizer,
    sent: &[StreamEvent],
    request: &ClientRequest,
    default_model: &ModelId,
) -> Result<Conclusion, ClientError> {
    let Some(stop) = normalizer.stop().cloned() else {
        return Err(ClientError::StreamTruncated);
    };
    let corrected = judge(
        &normalizer.held(),
        &stop,
        request.output(),
        !request.tools().is_empty(),
    )?;
    let mut tail = normalizer.close_all();
    tail.push(StreamEvent::Finish {
        stop_reason: corrected,
        usage: normalizer.usage().cloned(),
    });
    let model = normalizer
        .model()
        .cloned()
        .unwrap_or_else(|| default_model.clone());
    let step = replay(sent, &tail, model)?;
    Ok(Conclusion { tail, step })
}

fn replay(sent: &[StreamEvent], tail: &[StreamEvent], model: ModelId) -> Result<Step, ClientError> {
    let mut draft = StepDraft::new();
    for event in sent.iter().chain(tail) {
        draft
            .apply(event.clone())
            .map_err(|error| message_failure(&error))?;
    }
    draft
        .finish(Some(model))
        .map_err(|error| message_failure(&error))
}
