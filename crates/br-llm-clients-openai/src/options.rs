use br_llm_clients_api_openai_responses::{
    Map, Options, PromptCacheMode, ReasoningParameter, ReasoningSummary, ServiceTier, Value,
};

use crate::build_error::OpenAiBuildError;
use crate::profile::ModelProfile;

pub(crate) struct OptionInputs<'a> {
    pub(crate) profile: &'a ModelProfile,
    pub(crate) prompt_cache_key: Option<String>,
    pub(crate) service_tier: Option<ServiceTier>,
    pub(crate) cache_mode: Option<PromptCacheMode>,
    pub(crate) extra_body: Map<String, Value>,
}

pub(crate) fn dialect_options(inputs: OptionInputs<'_>) -> Result<Options, OpenAiBuildError> {
    let reasoning_parameter = inputs.profile.reasoning_parameter();
    let reasoning_summary = match reasoning_parameter {
        ReasoningParameter::Omitted => ReasoningSummary::None,
        _ => inputs.profile.reasoning_summary(),
    };
    let mut options = Options::default();
    options.reasoning_parameter = reasoning_parameter;
    options.reasoning_summary = reasoning_summary;
    options.prompt_cache_key = inputs.prompt_cache_key;
    options.service_tier = inputs.service_tier;
    options.cache_mode = inputs.cache_mode;
    options
        .with_extra_body(inputs.extra_body)
        .map_err(|error| OpenAiBuildError::InvalidExtraBody {
            reason: error.to_string(),
        })
}
