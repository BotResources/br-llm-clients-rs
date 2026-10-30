use br_llm_clients_api_openai_responses::{ReasoningParameter, ReasoningSummary};
use br_llm_clients_core::Capabilities;

#[derive(Debug, Clone, PartialEq)]
pub struct ModelProfile {
    capabilities: Capabilities,
    reasoning_parameter: ReasoningParameter,
    reasoning_summary: ReasoningSummary,
}

impl ModelProfile {
    pub fn new(
        capabilities: Capabilities,
        reasoning_parameter: ReasoningParameter,
        reasoning_summary: ReasoningSummary,
    ) -> Self {
        Self {
            capabilities,
            reasoning_parameter,
            reasoning_summary,
        }
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub fn reasoning_parameter(&self) -> ReasoningParameter {
        self.reasoning_parameter
    }

    pub fn reasoning_summary(&self) -> ReasoningSummary {
        self.reasoning_summary
    }

    pub(crate) fn into_capabilities(self) -> Capabilities {
        self.capabilities
    }

    pub fn with_summary(mut self, reasoning_summary: ReasoningSummary) -> Self {
        self.reasoning_summary = reasoning_summary;
        self
    }
}
