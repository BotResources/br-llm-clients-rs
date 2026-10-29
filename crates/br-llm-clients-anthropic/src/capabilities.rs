use std::num::NonZeroU32;

use br_llm_clients_core::{Accepted, Capabilities, CapabilitiesSpec, ReasoningLevel};

const OUTPUT_LIMIT: u32 = 128_000;

const OUTPUT_CEILING: NonZeroU32 = NonZeroU32::MIN.saturating_add(OUTPUT_LIMIT.saturating_sub(1));

const WITHOUT_OFF: [ReasoningLevel; 5] = [
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

const WITH_OFF: [ReasoningLevel; 6] = [
    ReasoningLevel::Off,
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

pub(crate) fn capabilities_of(model: &str) -> Capabilities {
    match model {
        "claude-fable-5-1" | "claude-opus-5-5" | "claude-sonnet-5-5" => declared(&WITHOUT_OFF),
        "claude-haiku-5-5" => declared(&WITH_OFF),
        _ => Capabilities::everything(),
    }
}

fn declared(levels: &[ReasoningLevel]) -> Capabilities {
    Capabilities::normalized(CapabilitiesSpec {
        reasoning_levels: Accepted::Only(levels.to_vec()),
        max_output_tokens: CapabilitiesSpec::max_output_tokens_of(OUTPUT_CEILING),
        structured_output_with_tools: false,
        ..CapabilitiesSpec::everything()
    })
}
