use std::time::Duration;

use br_llm_messages::{ModelName, Usage};
use tokio::time::Instant;

pub(crate) struct CallRecord {
    pub(crate) started: Instant,
    pub(crate) request_id: Option<String>,
    pub(crate) usage: Option<Usage>,
    pub(crate) model: Option<ModelName>,
    pub(crate) answered_model: Option<ModelName>,
    pub(crate) time_to_first_event: Option<Duration>,
}

impl CallRecord {
    pub(crate) fn start() -> Self {
        Self {
            started: Instant::now(),
            request_id: None,
            usage: None,
            model: None,
            answered_model: None,
            time_to_first_event: None,
        }
    }
}
