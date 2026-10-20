use br_llm_clients_core::{Accepted, Capabilities, CapabilitiesSpec, ReasoningLevel};

use crate::kimi_thinking::KimiThinking;

pub(crate) fn capabilities(kind: Option<&KimiThinking>) -> Capabilities {
    Capabilities::normalized(CapabilitiesSpec {
        reasoning_levels: Accepted::Only(levels(kind)),
        ..CapabilitiesSpec::everything()
    })
}

fn levels(kind: Option<&KimiThinking>) -> Vec<ReasoningLevel> {
    let above_off = [
        ReasoningLevel::Low,
        ReasoningLevel::Medium,
        ReasoningLevel::High,
        ReasoningLevel::Extra,
        ReasoningLevel::Max,
    ];
    match kind {
        Some(KimiThinking::EffortScale | KimiThinking::AlwaysThinking) => above_off.to_vec(),
        Some(KimiThinking::ThinkingSwitch) | None => {
            let mut levels = vec![ReasoningLevel::Off];
            levels.extend(above_off);
            levels
        }
    }
}
