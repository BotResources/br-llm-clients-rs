use serde_json::{Map, Value, json};

use crate::test_support::{options, settings};
use crate::{
    ChatCompletionsDialect, ChatCompletionsOptions, MaxTokensField, OptionsError, ReasoningLevel,
    ToolOnlyContent,
};

const RESERVED_FIELDS: [&str; 6] = [
    "content",
    "refusal",
    "tool_calls",
    "role",
    "audio",
    "function_call",
];

fn build(options: ChatCompletionsOptions) -> Result<ChatCompletionsDialect, OptionsError> {
    ChatCompletionsDialect::new(settings(ReasoningLevel::Off), options)
}

fn body(entries: Value) -> Map<String, Value> {
    entries.as_object().cloned().expect("object")
}

#[test]
fn given_a_blank_carry_back_field_when_built_then_it_is_refused() {
    for field in ["", " ", "\t\n"] {
        assert_eq!(
            build(options().with_reasoning_carry_back_field(field)).err(),
            Some(OptionsError::BlankCarryBackField)
        );
    }
}

#[test]
fn given_each_reserved_name_as_carry_back_field_when_built_then_it_is_refused_and_named() {
    for field in RESERVED_FIELDS {
        assert_eq!(
            build(options().with_reasoning_carry_back_field(field)).err(),
            Some(OptionsError::ReservedCarryBackField {
                field: field.to_owned()
            })
        );
    }
}

#[test]
fn given_the_usual_carry_back_names_when_built_then_they_are_accepted() {
    for field in ["reasoning_content", "reasoning", "thinking"] {
        assert!(build(options().with_reasoning_carry_back_field(field)).is_ok());
    }
}

#[test]
fn given_the_two_reasoning_names_when_they_differ_or_are_equal_then_both_are_accepted() {
    assert!(
        build(
            options()
                .with_reasoning_field("reasoning_content")
                .with_reasoning_carry_back_field("reasoning")
        )
        .is_ok()
    );
    assert!(
        build(
            options()
                .with_reasoning_field("reasoning_content")
                .with_reasoning_carry_back_field("reasoning_content")
        )
        .is_ok()
    );
}

#[test]
fn given_a_blank_empty_tool_result_text_when_built_then_it_is_refused() {
    for text in ["", "  ", "\n\t"] {
        assert_eq!(
            build(options().with_empty_tool_result_text(text)).err(),
            Some(OptionsError::BlankEmptyToolResultText)
        );
    }
    assert!(build(options().with_empty_tool_result_text("(no output)")).is_ok());
}

#[test]
fn given_max_tokens_in_the_extra_body_when_the_field_is_the_default_then_it_is_accepted() {
    let extra = body(json!({"max_tokens": 5}));
    assert!(build(options().with_extra_body(extra.clone())).is_ok());
    assert!(
        build(
            options()
                .with_max_tokens_field(MaxTokensField::MaxCompletionTokens)
                .with_extra_body(extra)
        )
        .is_ok()
    );
}

#[test]
fn given_max_tokens_in_the_extra_body_when_the_field_is_max_tokens_then_it_is_refused_whatever_the_call_order()
 {
    let extra = body(json!({"temperature": 0.1, "max_tokens": 5}));
    let expected = Some(OptionsError::ReservedExtraBodyMember {
        member: "max_tokens".to_owned(),
    });
    assert_eq!(
        build(
            options()
                .with_extra_body(extra.clone())
                .with_max_tokens_field(MaxTokensField::MaxTokens)
        )
        .err(),
        expected
    );
    assert_eq!(
        build(
            options()
                .with_max_tokens_field(MaxTokensField::MaxTokens)
                .with_extra_body(extra)
        )
        .err(),
        expected
    );
}

#[test]
fn given_the_field_max_tokens_when_the_extra_body_holds_the_other_name_then_it_is_still_refused() {
    let options = options()
        .with_max_tokens_field(MaxTokensField::MaxTokens)
        .with_extra_body(body(json!({"max_completion_tokens": 5})));
    assert_eq!(
        build(options).err(),
        Some(OptionsError::ReservedExtraBodyMember {
            member: "max_completion_tokens".to_owned()
        })
    );
}

#[test]
fn given_a_second_extra_body_when_set_then_it_replaces_the_first() {
    let replaced = options()
        .with_extra_body(body(json!({"a": 1})))
        .with_extra_body(body(json!({"b": 2})));
    assert_eq!(replaced, options().with_extra_body(body(json!({"b": 2}))));
}

#[test]
fn given_each_new_option_when_it_differs_then_the_options_are_not_equal() {
    let base = options();
    let different = [
        base.clone()
            .with_max_tokens_field(MaxTokensField::MaxTokens),
        base.clone()
            .with_reasoning_carry_back_field("reasoning_content"),
        base.clone().with_tool_message_name(true),
        base.clone().with_omit_effort_above_off(true),
        base.clone()
            .with_tool_only_content(ToolOnlyContent::EmptyText),
        base.clone().with_empty_tool_result_text("none"),
    ];
    for changed in different {
        assert_ne!(changed, base);
        assert_eq!(changed.clone(), changed);
    }
}

#[test]
fn given_the_defaults_set_by_hand_when_compared_then_the_options_are_equal() {
    assert_eq!(
        options()
            .with_max_tokens_field(MaxTokensField::MaxCompletionTokens)
            .with_tool_message_name(false)
            .with_omit_effort_above_off(false)
            .with_tool_only_content(ToolOnlyContent::Omitted),
        options()
    );
}

#[test]
fn given_the_options_when_debug_printed_then_the_six_new_fields_are_shown() {
    let shown = format!(
        "{:?}",
        options()
            .with_max_tokens_field(MaxTokensField::MaxTokens)
            .with_reasoning_carry_back_field("reasoning_content")
            .with_tool_message_name(true)
            .with_omit_effort_above_off(true)
            .with_tool_only_content(ToolOnlyContent::EmptyText)
            .with_empty_tool_result_text("none")
    );
    for expected in [
        "max_tokens_field: MaxTokens",
        "reasoning_carry_back_field: Some(\"reasoning_content\")",
        "tool_message_name: true",
        "omit_effort_above_off: true",
        "tool_only_content: EmptyText",
        "empty_tool_result_text: Some(\"none\")",
    ] {
        assert!(shown.contains(expected), "{expected} in {shown}");
    }
}

#[test]
fn given_the_new_options_errors_when_displayed_then_they_name_their_field_and_no_value() {
    let blank = OptionsError::BlankCarryBackField.to_string();
    assert!(blank.contains("carry-back"));
    let reserved = OptionsError::ReservedCarryBackField {
        field: "content".to_owned(),
    }
    .to_string();
    assert!(reserved.contains("content"));
    let empty = OptionsError::BlankEmptyToolResultText.to_string();
    assert!(empty.contains("empty tool result"));
    for error in [
        OptionsError::BlankCarryBackField,
        OptionsError::BlankEmptyToolResultText,
    ] {
        let as_error: &dyn std::error::Error = &error;
        assert!(as_error.source().is_none());
    }
}

#[test]
fn given_several_broken_new_rules_when_built_then_one_of_the_cases_is_given() {
    let broken = options()
        .with_reasoning_carry_back_field("")
        .with_empty_tool_result_text(" ");
    assert!(matches!(
        build(broken).err(),
        Some(OptionsError::BlankCarryBackField | OptionsError::BlankEmptyToolResultText)
    ));
}
