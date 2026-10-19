use br_llm_clients_core::ReasoningLevel;

use crate::test_support::{options, settings};
use crate::{
    ChatCompletionsDialect, ChatCompletionsOptions, OptionsError, ReasoningEffort, ReasoningRule,
    ReasoningTable,
};

use ReasoningEffort::{High, Low, Max, Medium, Minimal, Xhigh};
use ReasoningRule::{Omit, Refuse, Send};

const NONE: ReasoningEffort = ReasoningEffort::None;

fn build(table: ReasoningTable) -> Result<ChatCompletionsDialect, OptionsError> {
    ChatCompletionsDialect::new(
        settings(ReasoningLevel::Off),
        ChatCompletionsOptions::new(table),
    )
}

fn table(rules: [ReasoningRule; 6]) -> ReasoningTable {
    let [off, low, medium, high, extra, max] = rules;
    ReasoningTable::new(off, low, medium, high, extra, max)
}

#[test]
fn given_the_two_named_tables_when_the_dialect_is_built_then_it_is_accepted() {
    assert!(build(ReasoningTable::openai_effort()).is_ok());
    assert!(build(ReasoningTable::openai_effort_without_none()).is_ok());
}

#[test]
fn given_the_efforts_when_compared_then_they_rise_as_the_reference_lists_them() {
    let ascending = [NONE, Minimal, Low, Medium, High, Xhigh, Max];
    for pair in ascending.windows(2) {
        assert!(pair[0] < pair[1]);
    }
}

#[test]
fn given_the_named_tables_when_compared_with_the_hand_built_rows_then_they_are_equal() {
    assert_eq!(
        ReasoningTable::openai_effort(),
        table([
            Send(NONE),
            Send(Low),
            Send(Medium),
            Send(High),
            Send(Xhigh),
            Send(Max)
        ])
    );
    assert_eq!(
        ReasoningTable::openai_effort_without_none(),
        table([
            Refuse,
            Send(Low),
            Send(Medium),
            Send(High),
            Send(High),
            Send(High)
        ])
    );
}

#[test]
fn given_off_mapped_to_an_effort_above_none_when_built_then_the_dialect_is_refused() {
    for effort in [Minimal, Low, High, Max] {
        let broken = table([Send(effort), Refuse, Refuse, Refuse, Refuse, Refuse]);
        assert_eq!(
            build(broken).err(),
            Some(OptionsError::OffAboveNone { effort })
        );
    }
}

#[test]
fn given_off_that_sends_none_omits_or_refuses_when_built_then_it_is_accepted() {
    for off in [Send(NONE), Omit, Refuse] {
        let rules = [
            off,
            Send(Low),
            Send(Medium),
            Send(High),
            Send(Xhigh),
            Send(Max),
        ];
        assert!(build(table(rules)).is_ok());
    }
}

#[test]
fn given_omit_on_any_level_above_off_when_built_then_that_level_is_named() {
    let levels = [
        ReasoningLevel::Low,
        ReasoningLevel::Medium,
        ReasoningLevel::High,
        ReasoningLevel::Extra,
        ReasoningLevel::Max,
    ];
    for (position, level) in levels.into_iter().enumerate() {
        let mut rules = [
            Send(NONE),
            Send(Low),
            Send(Medium),
            Send(High),
            Send(Xhigh),
            Send(Max),
        ];
        *rules.get_mut(position + 1).expect("row") = Omit;
        assert_eq!(
            build(table(rules)).err(),
            Some(OptionsError::OmitAboveOff { level })
        );
    }
}

#[test]
fn given_none_on_any_level_above_off_when_built_then_that_level_is_named() {
    let levels = [
        ReasoningLevel::Low,
        ReasoningLevel::Medium,
        ReasoningLevel::High,
        ReasoningLevel::Extra,
        ReasoningLevel::Max,
    ];
    for (position, level) in levels.into_iter().enumerate() {
        let mut rules = [Refuse; 6];
        *rules.get_mut(position + 1).expect("row") = Send(NONE);
        assert_eq!(
            build(table(rules)).err(),
            Some(OptionsError::NoneAboveOff { level })
        );
    }
}

#[test]
fn given_a_higher_level_with_a_lower_effort_when_built_then_the_pair_is_named() {
    let broken = table([
        Send(NONE),
        Send(High),
        Send(Low),
        Send(High),
        Send(High),
        Send(High),
    ]);
    assert_eq!(
        build(broken).err(),
        Some(OptionsError::ReasoningTableDecreasing {
            level: ReasoningLevel::Medium,
            earlier: ReasoningLevel::Low
        })
    );
}

#[test]
fn given_a_decrease_hidden_behind_a_refused_level_when_built_then_it_is_still_found() {
    let broken = table([
        Send(NONE),
        Send(High),
        Refuse,
        Send(Medium),
        Send(High),
        Send(High),
    ]);
    assert_eq!(
        build(broken).err(),
        Some(OptionsError::ReasoningTableDecreasing {
            level: ReasoningLevel::High,
            earlier: ReasoningLevel::Low
        })
    );
}

#[test]
fn given_a_decrease_after_a_plateau_when_built_then_it_is_found_against_the_highest_so_far() {
    let broken = table([
        Refuse,
        Send(Low),
        Send(High),
        Send(High),
        Send(Medium),
        Send(Max),
    ]);
    assert_eq!(
        build(broken).err(),
        Some(OptionsError::ReasoningTableDecreasing {
            level: ReasoningLevel::Extra,
            earlier: ReasoningLevel::Medium
        })
    );
}

#[test]
fn given_equal_efforts_on_several_levels_when_built_then_it_is_accepted() {
    let flat = table([
        Refuse,
        Send(Minimal),
        Send(Minimal),
        Send(Minimal),
        Send(Minimal),
        Send(Minimal),
    ]);
    assert!(build(flat).is_ok());
}

#[test]
fn given_a_table_that_refuses_the_level_of_the_settings_when_built_then_only_encode_refuses() {
    let refusing = ChatCompletionsOptions::new(ReasoningTable::openai_effort_without_none());
    let dialect = ChatCompletionsDialect::new(settings(ReasoningLevel::Off), refusing);
    assert!(dialect.is_ok());
}

#[test]
fn given_every_options_error_when_displayed_then_it_names_its_level_field_or_member() {
    let cases = [
        (
            OptionsError::ReasoningTableDecreasing {
                level: ReasoningLevel::Medium,
                earlier: ReasoningLevel::Low,
            },
            "Medium",
        ),
        (OptionsError::OffAboveNone { effort: High }, "High"),
        (
            OptionsError::OmitAboveOff {
                level: ReasoningLevel::Extra,
            },
            "Extra",
        ),
        (
            OptionsError::NoneAboveOff {
                level: ReasoningLevel::Max,
            },
            "Max",
        ),
        (OptionsError::BlankReasoningField, "reasoning field"),
        (
            OptionsError::ReservedReasoningField {
                field: "content".to_owned(),
            },
            "content",
        ),
        (OptionsError::BlankCacheReadField, "cache read field"),
        (
            OptionsError::ReservedExtraBodyMember {
                member: "tools".to_owned(),
            },
            "tools",
        ),
    ];
    for (error, name) in cases {
        assert!(error.to_string().contains(name), "{error}");
        let as_error: &dyn std::error::Error = &error;
        assert!(as_error.source().is_none());
    }
}

#[test]
fn given_the_default_options_helper_when_built_then_it_uses_the_effort_table() {
    assert!(build(ReasoningTable::openai_effort()).is_ok());
    assert_eq!(
        options(),
        ChatCompletionsOptions::new(ReasoningTable::openai_effort())
    );
}
