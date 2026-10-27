use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::{ResponseDraft, ResponseEnding};

use crate::failure_map::message_failure;
use crate::judge::judge;
use crate::normalizer::Normalizer;

pub(crate) fn decide(
    normalizer: &Normalizer,
    draft: &ResponseDraft,
    request: &ClientRequest,
) -> Result<ResponseEnding, ClientError> {
    let Some(stop) = normalizer.stop() else {
        return Err(ClientError::StreamTruncated);
    };
    let ending = judge(
        &normalizer.held(),
        stop,
        request.output(),
        !request.tools().is_empty(),
    )?;
    rehearse(normalizer, draft, &ending)?;
    Ok(ending)
}

fn rehearse(
    normalizer: &Normalizer,
    draft: &ResponseDraft,
    ending: &ResponseEnding,
) -> Result<(), ClientError> {
    let mut draft = draft.clone();
    let mut normalizer = normalizer.rehearsal();
    for delta in normalizer.close_all() {
        draft
            .apply(&delta)
            .map_err(|error| message_failure(&error))?;
    }
    draft
        .apply(&normalizer.completed(ending.clone()))
        .map(drop)
        .map_err(|error| message_failure(&error))
}
