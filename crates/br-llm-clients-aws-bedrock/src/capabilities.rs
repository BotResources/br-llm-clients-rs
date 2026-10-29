use std::num::NonZeroU32;

use br_llm_clients_core::{Accepted, Capabilities, CapabilitiesSpec, ReasoningLevel};

const MAX_OUTPUT_TOKENS: NonZeroU32 = match NonZeroU32::new(128_000) {
    Some(limit) => limit,
    None => panic!("128000 is not zero"),
};

const PROFILE_PREFIXES: [&str; 7] = ["global.", "us.", "eu.", "au.", "jp.", "apac.", "us-gov."];
const PROVIDER_PREFIX: &str = "anthropic.";

const THINKING_ONLY: [&str; 3] = ["claude-fable-5-1", "claude-opus-5-5", "claude-sonnet-5-5"];
const THINKING_OPTIONAL: &str = "claude-haiku-5-5";

const THINKING_LEVELS: [ReasoningLevel; 5] = [
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

pub(crate) fn of_model(model: &str) -> Capabilities {
    let base = base_name(model);
    if THINKING_ONLY.contains(&base) {
        current(THINKING_LEVELS.to_vec())
    } else if base == THINKING_OPTIONAL {
        let mut levels = vec![ReasoningLevel::Off];
        levels.extend(THINKING_LEVELS);
        current(levels)
    } else {
        Capabilities::normalized(CapabilitiesSpec {
            caching: false,
            ..CapabilitiesSpec::everything()
        })
    }
}

fn base_name(model: &str) -> &str {
    let unprefixed = PROFILE_PREFIXES
        .iter()
        .find_map(|prefix| model.strip_prefix(prefix))
        .unwrap_or(model);
    unprefixed
        .strip_prefix(PROVIDER_PREFIX)
        .unwrap_or(unprefixed)
}

fn current(levels: Vec<ReasoningLevel>) -> Capabilities {
    Capabilities::normalized(CapabilitiesSpec {
        reasoning_levels: Accepted::Only(levels),
        max_output_tokens: CapabilitiesSpec::max_output_tokens_of(MAX_OUTPUT_TOKENS),
        structured_output_with_tools: false,
        caching: false,
        ..CapabilitiesSpec::everything()
    })
}
