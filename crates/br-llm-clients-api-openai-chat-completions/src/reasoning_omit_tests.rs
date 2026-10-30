use br_llm_clients_core::ReasoningLevel;
use serde_json::Value;

use crate::test_support::{dialect_for, encoded_body, hello, settings};
use crate::{
    ChatCompletionsDialect, ChatCompletionsOptions, OptionsError, ReasoningEffort, ReasoningRule,
    ReasoningTable,
};

use ReasoningEffort::{High, Low, Medium};
use ReasoningRule::{Omit, Refuse, Send};

const NONE: ReasoningEffort = ReasoningEffort::None;
const LEVELS: [ReasoningLevel; 6] = [
    ReasoningLevel::Off,
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

fn table(rules: [ReasoningRule; 6]) -> ReasoningTable {
    let [off, low, medium, high, extra, max] = rules;
    ReasoningTable::new(off, low, medium, high, extra, max)
}

fn build(
    table: ReasoningTable,
    omit_above_off: bool,
) -> Result<ChatCompletionsDialect, OptionsError> {
    ChatCompletionsDialect::new(
        settings(ReasoningLevel::Off),
        ChatCompletionsOptions::new(table).with_omit_effort_above_off(omit_above_off),
    )
}

fn omitting_at(index: usize) -> ReasoningTable {
    let mut rules = [
        Send(NONE),
        Send(Low),
        Send(Medium),
        Send(High),
        Send(High),
        Send(High),
    ];
    if let Some(rule) = rules.get_mut(index) {
        *rule = Omit;
    }
    table(rules)
}

#[test]
fn given_omit_on_a_level_above_off_when_the_switch_is_on_then_every_level_is_accepted() {
    for index in 1..LEVELS.len() {
        assert!(build(omitting_at(index), true).is_ok(), "{index}");
    }
    assert!(build(table([Omit; 6]), true).is_ok());
}

#[test]
fn given_omit_on_a_level_above_off_when_the_switch_is_off_then_that_level_is_named() {
    for (index, level) in LEVELS.iter().enumerate().skip(1) {
        assert_eq!(
            build(omitting_at(index), false).err(),
            Some(OptionsError::OmitAboveOff { level: *level })
        );
    }
}

#[test]
fn given_the_default_options_when_a_table_omits_above_off_then_it_is_refused() {
    let default_options = ChatCompletionsOptions::new(omitting_at(3));
    assert_eq!(
        ChatCompletionsDialect::new(settings(ReasoningLevel::Off), default_options).err(),
        Some(OptionsError::OmitAboveOff {
            level: ReasoningLevel::High
        })
    );
}

#[test]
fn given_the_switch_on_when_the_send_values_decrease_around_an_omit_then_the_order_is_still_checked()
 {
    let decreasing = table([Send(NONE), Send(High), Omit, Send(Low), Refuse, Refuse]);
    assert_eq!(
        build(decreasing, true).err(),
        Some(OptionsError::ReasoningTableDecreasing {
            level: ReasoningLevel::High,
            earlier: ReasoningLevel::Low
        })
    );
}

#[test]
fn given_the_switch_on_when_an_omit_sits_between_rising_sends_then_it_takes_no_part_in_the_order() {
    let rising = table([Send(NONE), Send(Low), Omit, Send(Low), Omit, Send(High)]);
    assert!(build(rising, true).is_ok());
}

#[test]
fn given_the_switch_on_when_a_level_above_off_sends_none_then_it_is_still_refused() {
    let none_above = table([Omit, Omit, Send(NONE), Omit, Omit, Omit]);
    assert_eq!(
        build(none_above, true).err(),
        Some(OptionsError::NoneAboveOff {
            level: ReasoningLevel::Medium
        })
    );
}

#[test]
fn given_the_switch_on_when_off_sends_an_effort_above_none_then_it_is_still_refused() {
    let off_high = table([Send(High), Omit, Omit, Omit, Omit, Omit]);
    assert_eq!(
        build(off_high, true).err(),
        Some(OptionsError::OffAboveNone { effort: High })
    );
}

#[test]
fn given_the_switch_on_when_encoding_an_omitted_level_above_off_then_no_effort_is_written() {
    let options = ChatCompletionsOptions::new(omitting_at(3)).with_omit_effort_above_off(true);
    let omitted = encoded_body(
        &dialect_for(ReasoningLevel::High, options.clone()),
        &hello(),
    );
    assert!(omitted.get("reasoning_effort").is_none());
    let sent = encoded_body(&dialect_for(ReasoningLevel::Medium, options), &hello());
    assert_eq!(sent["reasoning_effort"], Value::from("medium"));
}
