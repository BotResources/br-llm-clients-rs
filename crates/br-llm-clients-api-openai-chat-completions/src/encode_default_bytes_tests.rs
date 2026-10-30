use crate::golden_cases::{GoldenCase, golden_cases};
use crate::test_support::{dialect_for, encode};
use crate::{ChatCompletionsOptions, MaxTokensField, ToolOnlyContent};

fn golden(name: &str) -> &'static str {
    match name {
        "hello" => include_str!("golden/hello.json"),
        "system_and_images" => include_str!("golden/system_and_images.json"),
        "tool_loop_with_tools" => include_str!("golden/tool_loop_with_tools.json"),
        "tool_loop_extra_body" => include_str!("golden/tool_loop_extra_body.json"),
        "structured_with_tools" => include_str!("golden/structured_with_tools.json"),
        "two_texts" => include_str!("golden/two_texts.json"),
        other => panic!("no golden file for {other}"),
    }
}

fn bytes_of(case: &GoldenCase, options: ChatCompletionsOptions) -> Vec<u8> {
    encode(&dialect_for(case.level, options), &case.request)
        .expect("encoded")
        .body()
        .to_vec()
}

fn with_every_defaulted_option_set_by_hand(
    options: ChatCompletionsOptions,
) -> ChatCompletionsOptions {
    options
        .with_max_tokens_field(MaxTokensField::MaxCompletionTokens)
        .with_tool_message_name(false)
        .with_omit_effort_above_off(false)
        .with_tool_only_content(ToolOnlyContent::Omitted)
}

#[test]
fn given_the_options_of_0_1_when_encoding_the_recorded_requests_then_the_bytes_are_those_of_0_1() {
    let cases = golden_cases();
    assert_eq!(cases.len(), 6);
    for case in cases {
        assert_eq!(
            bytes_of(&case, case.options.clone()),
            golden(case.name).as_bytes(),
            "{}",
            case.name
        );
    }
}

#[test]
fn given_the_four_switchable_new_options_set_to_their_defaults_by_hand_when_encoding_then_the_bytes_are_those_of_0_1()
 {
    for case in golden_cases() {
        let options = with_every_defaulted_option_set_by_hand(case.options.clone());
        assert_eq!(
            bytes_of(&case, options),
            golden(case.name).as_bytes(),
            "{}",
            case.name
        );
    }
}

#[test]
fn given_the_defaults_when_encoding_twice_then_the_bytes_are_equal() {
    for case in golden_cases() {
        assert_eq!(
            bytes_of(&case, case.options.clone()),
            bytes_of(&case, case.options.clone())
        );
    }
}
