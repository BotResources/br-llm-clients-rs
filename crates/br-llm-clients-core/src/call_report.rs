use std::time::Duration;

use br_llm_messages::{ModelId, Usage};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallReport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    usage: Option<Usage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<ModelId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    time_to_first_event: Option<Duration>,
    total_duration: Duration,
}

impl CallReport {
    pub fn new(total_duration: Duration) -> Self {
        Self {
            usage: None,
            model: None,
            request_id: None,
            time_to_first_event: None,
            total_duration,
        }
    }

    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn with_model(mut self, model: ModelId) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_time_to_first_event(mut self, time_to_first_event: Duration) -> Self {
        self.time_to_first_event = Some(time_to_first_event);
        self
    }

    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    pub fn model(&self) -> Option<&ModelId> {
        self.model.as_ref()
    }

    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub fn time_to_first_event(&self) -> Option<Duration> {
        self.time_to_first_event
    }

    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }
}
