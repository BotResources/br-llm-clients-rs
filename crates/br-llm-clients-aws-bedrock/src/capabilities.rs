use std::num::NonZeroU32;

use br_llm_clients_core::{Accepted, Capabilities, CapabilitiesSpec, ReasoningLevel};

const TOKENS_128_000: NonZeroU32 = match NonZeroU32::new(128_000) {
    Some(limit) => limit,
    None => panic!("128000 is not zero"),
};

const TOKENS_64_000: NonZeroU32 = match NonZeroU32::new(64_000) {
    Some(limit) => limit,
    None => panic!("64000 is not zero"),
};

const PROFILE_PREFIXES: [&str; 7] = ["global.", "us.", "eu.", "au.", "jp.", "apac.", "us-gov."];
const PROVIDER_PREFIX: &str = "anthropic.";

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

const OFF_ONLY: [ReasoningLevel; 1] = [ReasoningLevel::Off];

struct Row {
    base_name: &'static str,
    levels: &'static [ReasoningLevel],
    max_output_tokens: NonZeroU32,
    caching: bool,
}

const TABLE: [Row; 7] = [
    Row {
        base_name: "claude-fable-5-1",
        levels: &FROM_LOW_TO_MAX,
        max_output_tokens: TOKENS_128_000,
        caching: false,
    },
    Row {
        base_name: "claude-opus-5-5",
        levels: &FROM_LOW_TO_MAX,
        max_output_tokens: TOKENS_128_000,
        caching: true,
    },
    Row {
        base_name: "claude-sonnet-5-5",
        levels: &FROM_LOW_TO_MAX,
        max_output_tokens: TOKENS_128_000,
        caching: true,
    },
    Row {
        base_name: "claude-haiku-5-5",
        levels: &FROM_OFF_TO_MAX,
        max_output_tokens: TOKENS_128_000,
        caching: true,
    },
    Row {
        base_name: "claude-opus-4-8",
        levels: &FROM_OFF_TO_MAX,
        max_output_tokens: TOKENS_128_000,
        caching: true,
    },
    Row {
        base_name: "claude-opus-4-7",
        levels: &FROM_OFF_TO_MAX,
        max_output_tokens: TOKENS_128_000,
        caching: true,
    },
    Row {
        base_name: "claude-haiku-4-5-20251001-v1:0",
        levels: &OFF_ONLY,
        max_output_tokens: TOKENS_64_000,
        caching: true,
    },
];

pub(crate) fn of_model(model: &str) -> Capabilities {
    let base = base_name(model);
    match TABLE.iter().find(|row| row.base_name == base) {
        Some(row) => row.capabilities(),
        None => Capabilities::normalized(CapabilitiesSpec {
            caching: false,
            ..CapabilitiesSpec::everything()
        }),
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

impl Row {
    fn capabilities(&self) -> Capabilities {
        Capabilities::normalized(CapabilitiesSpec {
            reasoning_levels: Accepted::Only(self.levels.to_vec()),
            max_output_tokens: CapabilitiesSpec::max_output_tokens_of(self.max_output_tokens),
            structured_output_with_tools: false,
            caching: self.caching,
            ..CapabilitiesSpec::everything()
        })
    }
}
