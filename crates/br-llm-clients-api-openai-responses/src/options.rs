use std::fmt;

use serde_json::{Map, Value};

use crate::option_values::{PromptCacheMode, ServiceTier};
use crate::options_error::OptionsError;

const RESERVED_MEMBERS: [&str; 13] = [
    "model",
    "stream",
    "store",
    "instructions",
    "input",
    "max_output_tokens",
    "tools",
    "text",
    "reasoning",
    "include",
    "prompt_cache_key",
    "service_tier",
    "prompt_cache_options",
];

fn reserved_member(extra_body: &Map<String, Value>) -> Option<String> {
    RESERVED_MEMBERS
        .iter()
        .find(|member| extra_body.contains_key(**member))
        .map(|member| (*member).to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ReasoningSummary {
    None,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ReasoningParameter {
    #[default]
    Sent,
    Omitted,
}

#[derive(Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Options {
    pub reasoning_summary: ReasoningSummary,
    pub reasoning_parameter: ReasoningParameter,
    pub prompt_cache_key: Option<String>,
    pub service_tier: Option<ServiceTier>,
    pub cache_mode: Option<PromptCacheMode>,
    pub(crate) extra_body: Map<String, Value>,
}

impl Options {
    pub fn with_extra_body(mut self, extra_body: Map<String, Value>) -> Result<Self, OptionsError> {
        if let Some(member) = reserved_member(&extra_body) {
            return Err(OptionsError::ReservedExtraBodyMember { member });
        }
        self.extra_body = extra_body;
        Ok(self)
    }

    pub fn extra_body(&self) -> &Map<String, Value> {
        &self.extra_body
    }
}

impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cache_key = if self.prompt_cache_key.is_some() {
            "set"
        } else {
            "unset"
        };
        let members: Vec<&String> = self.extra_body.keys().collect();
        f.debug_struct("Options")
            .field("reasoning_summary", &self.reasoning_summary)
            .field("reasoning_parameter", &self.reasoning_parameter)
            .field("prompt_cache_key", &cache_key)
            .field("service_tier", &self.service_tier)
            .field("cache_mode", &self.cache_mode)
            .field("extra_body", &members)
            .finish()
    }
}
