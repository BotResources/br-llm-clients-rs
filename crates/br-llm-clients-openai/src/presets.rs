use std::num::NonZeroU32;

use br_llm_clients_api_openai_responses::{ReasoningParameter, ReasoningSummary};
use br_llm_clients_core::{Accepted, Capabilities, CapabilitiesSpec, ReasoningLevel};

use crate::profile::ModelProfile;

const TOKENS_128_000: NonZeroU32 = NonZeroU32::new(128_000).unwrap();
const TOKENS_32_768: NonZeroU32 = NonZeroU32::new(32_768).unwrap();

const FROM_LOW_TO_MAX: [ReasoningLevel; 5] = [
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

const FROM_OFF_TO_MAX: [ReasoningLevel; 6] = [
    ReasoningLevel::Off,
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

const FROM_OFF_TO_EXTRA: [ReasoningLevel; 5] = [
    ReasoningLevel::Off,
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
];

const OFF_ONLY: [ReasoningLevel; 1] = [ReasoningLevel::Off];

fn preset(levels: &[ReasoningLevel], ceiling: NonZeroU32, caching: bool) -> ModelProfile {
    let spec = CapabilitiesSpec {
        reasoning_levels: Accepted::Only(levels.to_vec()),
        max_output_tokens: CapabilitiesSpec::max_output_tokens_of(ceiling),
        caching,
        ..CapabilitiesSpec::everything()
    };
    ModelProfile::new(
        Capabilities::normalized(spec),
        ReasoningParameter::Sent,
        ReasoningSummary::None,
    )
}

impl ModelProfile {
    pub fn gpt_6_astra() -> Self {
        preset(&FROM_LOW_TO_MAX, TOKENS_128_000, true)
    }

    pub fn gpt_6_1_sol() -> Self {
        preset(&FROM_LOW_TO_MAX, TOKENS_128_000, true)
    }

    pub fn gpt_6_luna() -> Self {
        preset(&FROM_OFF_TO_MAX, TOKENS_128_000, true)
    }

    pub fn gpt_5_6_sol() -> Self {
        preset(&FROM_OFF_TO_MAX, TOKENS_128_000, true)
    }

    pub fn gpt_5_5() -> Self {
        preset(&FROM_OFF_TO_EXTRA, TOKENS_128_000, true)
    }

    pub fn gpt_5_4() -> Self {
        preset(&FROM_OFF_TO_EXTRA, TOKENS_128_000, true)
    }

    pub fn gpt_5_4_mini() -> Self {
        preset(&FROM_OFF_TO_EXTRA, TOKENS_128_000, true)
    }

    pub fn gpt_4_1_mini() -> Self {
        Self {
            reasoning_parameter: ReasoningParameter::Omitted,
            ..preset(&OFF_ONLY, TOKENS_32_768, true)
        }
    }
}
