use br_llm_messages::{ModelResponse, ResponseDelta, ResponseDraft};

use crate::fold_error::FoldError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    position: usize,
    reason: String,
}

impl Stop {
    pub fn position(&self) -> usize {
        self.position
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Clone)]
pub struct PartialFold {
    draft: ResponseDraft,
    stop: Option<Stop>,
}

impl PartialFold {
    pub fn draft(&self) -> &ResponseDraft {
        &self.draft
    }

    pub fn into_draft(self) -> ResponseDraft {
        self.draft
    }

    pub fn stop(&self) -> Option<&Stop> {
        self.stop.as_ref()
    }
}

pub fn fold_partial(deltas: &[ResponseDelta]) -> Option<PartialFold> {
    let first = deltas.first()?;
    let mut draft = ResponseDraft::new(first.turn(), first.entry());
    for (position, delta) in deltas.iter().enumerate() {
        if let Err(error) = draft.apply(delta) {
            return Some(PartialFold {
                draft,
                stop: Some(Stop {
                    position,
                    reason: error.to_string(),
                }),
            });
        }
    }
    Some(PartialFold { draft, stop: None })
}

pub fn fold_deltas(deltas: &[ResponseDelta]) -> Result<ModelResponse, FoldError> {
    let folded = fold_partial(deltas)
        .ok_or_else(|| FoldError::new("there is no delta to fold".to_owned()))?;
    if let Some(stop) = &folded.stop {
        return Err(FoldError::new(format!(
            "delta {} is refused: {}",
            stop.position, stop.reason
        )));
    }
    folded
        .draft
        .into_response()
        .ok_or_else(|| FoldError::new("the deltas stop before ResponseCompleted".to_owned()))
}
