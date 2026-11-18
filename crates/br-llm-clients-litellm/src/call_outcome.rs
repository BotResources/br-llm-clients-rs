use br_llm_clients_core::CallOutcome;

#[derive(Debug, Clone, PartialEq)]
pub struct LiteLlmCallOutcome {
    outcome: CallOutcome,
    deployment_id: Option<String>,
}

impl LiteLlmCallOutcome {
    pub(crate) fn new(outcome: CallOutcome, deployment_id: Option<String>) -> Self {
        Self {
            outcome,
            deployment_id,
        }
    }

    pub fn outcome(&self) -> &CallOutcome {
        &self.outcome
    }

    pub fn deployment_id(&self) -> Option<&str> {
        self.deployment_id.as_deref()
    }

    pub fn into_parts(self) -> (CallOutcome, Option<String>) {
        (self.outcome, self.deployment_id)
    }
}
