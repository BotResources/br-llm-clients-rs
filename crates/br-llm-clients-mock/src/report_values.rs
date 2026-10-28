use std::time::Duration;

use br_llm_clients_core::CallReport;
use br_llm_messages::{ModelName, Usage};

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ReportValues {
    pub(crate) model: Option<ModelName>,
    pub(crate) request_id: Option<String>,
    pub(crate) time_to_first_event: Option<Duration>,
    pub(crate) total_duration: Option<Duration>,
    pub(crate) usage: Option<Usage>,
}

impl ReportValues {
    pub(crate) fn report(&self, usage: Option<Usage>, model: Option<ModelName>) -> CallReport {
        let mut report = CallReport::new(self.total_duration.unwrap_or(Duration::ZERO));
        if let Some(usage) = usage {
            report = report.with_usage(usage);
        }
        if let Some(model) = model {
            report = report.with_model(model);
        }
        if let Some(request_id) = &self.request_id {
            report = report.with_request_id(request_id.clone());
        }
        if let Some(time) = self.time_to_first_event {
            report = report.with_time_to_first_event(time);
        }
        report
    }

    pub(crate) fn named_report(&self) -> CallReport {
        self.report(self.usage.clone(), self.model.clone())
    }
}

pub(crate) fn bare_report() -> CallReport {
    CallReport::new(Duration::ZERO)
}
