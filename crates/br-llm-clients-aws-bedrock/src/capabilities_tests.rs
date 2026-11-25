use std::num::NonZeroU32;

use br_llm_clients_core::{Accepted, Capabilities, CapabilitiesSpec, ReasoningLevel};

use crate::capabilities::of_model;

const LOW_TO_MAX: [ReasoningLevel; 5] = [
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

const OFF_TO_MAX: [ReasoningLevel; 6] = [
    ReasoningLevel::Off,
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

const PROFILE_PREFIXES: [&str; 7] = ["global.", "us.", "eu.", "au.", "jp.", "apac.", "us-gov."];

fn declared(levels: &[ReasoningLevel], tokens: u32, caching: bool) -> Capabilities {
    Capabilities::normalized(CapabilitiesSpec {
        reasoning_levels: Accepted::Only(levels.to_vec()),
        max_output_tokens: CapabilitiesSpec::max_output_tokens_of(NonZeroU32::new(tokens).unwrap()),
        structured_output_with_tools: false,
        caching,
        ..CapabilitiesSpec::everything()
    })
}

fn undeclared() -> Capabilities {
    Capabilities::normalized(CapabilitiesSpec {
        caching: false,
        ..CapabilitiesSpec::everything()
    })
}

fn table() -> Vec<(&'static str, Capabilities)> {
    vec![
        ("claude-fable-5-1", declared(&LOW_TO_MAX, 128_000, false)),
        ("claude-opus-5-5", declared(&LOW_TO_MAX, 128_000, true)),
        ("claude-sonnet-5-5", declared(&LOW_TO_MAX, 128_000, true)),
        ("claude-haiku-5-5", declared(&OFF_TO_MAX, 128_000, true)),
        ("claude-opus-4-8", declared(&OFF_TO_MAX, 128_000, true)),
        ("claude-opus-4-7", declared(&OFF_TO_MAX, 128_000, true)),
        (
            "claude-haiku-4-5-20251001-v1:0",
            declared(&[ReasoningLevel::Off], 64_000, true),
        ),
    ]
}

#[test]
fn given_a_base_name_of_the_table_when_looked_up_then_its_row_is_declared() {
    for (base, expected) in table() {
        assert_eq!(of_model(base), expected, "{base}");
    }
}

#[test]
fn given_a_base_name_behind_any_profile_prefix_when_looked_up_then_its_row_is_declared() {
    for (base, expected) in table() {
        for prefix in PROFILE_PREFIXES {
            let profile = format!("{prefix}anthropic.{base}");
            assert_eq!(of_model(&profile), expected, "{profile}");
            let bare = format!("{prefix}{base}");
            assert_eq!(of_model(&bare), expected, "{bare}");
        }
    }
}

#[test]
fn given_a_base_model_id_when_looked_up_then_its_row_is_declared() {
    for (base, expected) in table() {
        let id = format!("anthropic.{base}");
        assert_eq!(of_model(&id), expected, "{id}");
    }
}

#[test]
fn given_the_table_when_read_then_only_fable_5_1_declares_no_cache() {
    let uncached: Vec<&str> = table()
        .into_iter()
        .filter(|(_, capabilities)| !capabilities.caching())
        .map(|(base, _)| base)
        .collect();
    assert_eq!(uncached, ["claude-fable-5-1"]);
}

#[test]
fn given_the_table_when_read_then_no_row_accepts_structured_output_with_tools() {
    for (base, _) in table() {
        assert!(!of_model(base).structured_output_with_tools(), "{base}");
    }
}

#[test]
fn given_an_id_outside_the_table_when_looked_up_then_everything_is_declared_without_cache() {
    let ids = [
        "eu.anthropic.claude-sonnet-4-6",
        "eu.anthropic.claude-opus-4-6-v1",
        "eu.anthropic.claude-opus-4-5-20251101-v1:0",
        "eu.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "eu.anthropic.claude-sonnet-4-20250514-v1:0",
        "eu.anthropic.claude-3-7-sonnet-20250219-v1:0",
        "eu.anthropic.claude-haiku-4-5",
        "eu.anthropic.claude-opus-6-0",
        "eu.anthropic.claude-does-not-exist",
        "claude-opus-5-5-v1",
        "CLAUDE-OPUS-5-5",
        "",
    ];
    for id in ids {
        assert_eq!(of_model(id), undeclared(), "{id}");
    }
}

#[test]
fn given_two_profile_prefixes_or_two_provider_prefixes_when_looked_up_then_the_id_is_outside_the_table()
 {
    let ids = [
        "eu.global.anthropic.claude-opus-5-5",
        "global.eu.claude-opus-5-5",
        "eu.anthropic.anthropic.claude-opus-5-5",
        "anthropic.eu.claude-opus-5-5",
    ];
    for id in ids {
        assert_eq!(of_model(id), undeclared(), "{id}");
    }
}
