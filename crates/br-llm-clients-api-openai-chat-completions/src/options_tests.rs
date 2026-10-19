use serde_json::{Map, Value, json};

use crate::test_support::{options, settings};
use crate::{ChatCompletionsDialect, ChatCompletionsOptions, OptionsError, ReasoningLevel};

fn build(options: ChatCompletionsOptions) -> Result<ChatCompletionsDialect, OptionsError> {
    ChatCompletionsDialect::new(settings(ReasoningLevel::Off), options)
}

fn body(entries: Value) -> Map<String, Value> {
    entries.as_object().cloned().expect("object")
}

#[test]
fn given_a_blank_reasoning_field_when_built_then_it_is_refused() {
    for field in ["", " ", "\t\n"] {
        assert_eq!(
            build(options().with_reasoning_field(field)).err(),
            Some(OptionsError::BlankReasoningField)
        );
    }
}

#[test]
fn given_each_field_the_decoder_already_reads_when_used_as_reasoning_field_then_it_is_refused() {
    for field in [
        "content",
        "refusal",
        "tool_calls",
        "role",
        "audio",
        "function_call",
    ] {
        assert_eq!(
            build(options().with_reasoning_field(field)).err(),
            Some(OptionsError::ReservedReasoningField {
                field: field.to_owned()
            })
        );
    }
}

#[test]
fn given_the_usual_reasoning_field_names_when_built_then_they_are_accepted() {
    for field in ["reasoning_content", "reasoning", "thinking"] {
        assert!(build(options().with_reasoning_field(field)).is_ok());
    }
}

#[test]
fn given_a_blank_cache_read_field_when_built_then_it_is_refused() {
    for field in ["", "  "] {
        assert_eq!(
            build(options().with_cache_read_field(field)).err(),
            Some(OptionsError::BlankCacheReadField)
        );
    }
    assert!(build(options().with_cache_read_field("cached_prompt_tokens")).is_ok());
}

#[test]
fn given_each_member_the_dialect_writes_when_used_in_the_extra_body_then_it_is_refused() {
    let reserved = [
        "model",
        "messages",
        "stream",
        "stream_options",
        "max_completion_tokens",
        "reasoning_effort",
        "tools",
        "response_format",
    ];
    for member in reserved {
        let extra = body(json!({ "temperature": 0.2, member: 1 }));
        assert_eq!(
            build(options().with_extra_body(extra)).err(),
            Some(OptionsError::ReservedExtraBodyMember {
                member: member.to_owned()
            })
        );
    }
}

#[test]
fn given_other_members_in_the_extra_body_when_built_then_they_are_accepted() {
    let extra = body(json!({"temperature": 0.2, "top_p": 0.9, "seed": 7, "tool_choice": "auto"}));
    assert!(build(options().with_extra_body(extra)).is_ok());
}

#[test]
fn given_an_empty_extra_body_and_no_fields_when_built_then_it_is_the_default() {
    assert_eq!(options().with_extra_body(Map::new()), options());
}

#[test]
fn given_several_broken_rules_when_built_then_one_of_the_cases_is_given() {
    let broken = options()
        .with_reasoning_field("")
        .with_cache_read_field("")
        .with_extra_body(body(json!({"model": "x"})));
    assert!(build(broken).is_err());
}

#[test]
fn given_a_clone_of_the_options_when_compared_then_it_is_equal() {
    let configured = options()
        .with_reasoning_field("reasoning_content")
        .with_cache_read_field("cached");
    assert_eq!(configured.clone(), configured);
    assert_ne!(configured, options());
}
