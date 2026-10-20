use br_llm_clients_common_call::Dialect;
use serde_json::{Map, Value, json};

use crate::test_support::{
    dialect_for, options, parameter, request_with, settings, shape, tool, user_text,
};
use crate::{
    ChatCompletionsDialect, ChatCompletionsOptions, MaxTokensField, OptionsError, ReasoningLevel,
};
use br_llm_clients_core::ParameterKind;

fn fullest_body(field: MaxTokensField) -> Map<String, Value> {
    let request = request_with(
        Some("be brief"),
        vec![user_text("hello")],
        vec![tool("weather", "gives the weather", vec![])],
        shape(
            "answer",
            None,
            vec![parameter("value", "", true, ParameterKind::String)],
        ),
    );
    let dialect = dialect_for(ReasoningLevel::High, options().with_max_tokens_field(field));
    let encoded = dialect.encode(&request).expect("encoded");
    serde_json::from_slice::<Value>(encoded.body())
        .expect("json")
        .as_object()
        .cloned()
        .expect("object")
}

fn refused(options: ChatCompletionsOptions) -> Option<OptionsError> {
    ChatCompletionsDialect::new(settings(ReasoningLevel::Off), options).err()
}

#[test]
fn given_the_fullest_request_when_each_member_written_is_used_in_the_extra_body_then_it_is_refused()
{
    for field in [
        MaxTokensField::MaxCompletionTokens,
        MaxTokensField::MaxTokens,
    ] {
        let body = fullest_body(field);
        assert_eq!(body.len(), 8, "{body:?}");
        for member in body.keys() {
            let mut extra = Map::new();
            extra.insert(member.clone(), json!(1));
            assert_eq!(
                refused(
                    options()
                        .with_max_tokens_field(field)
                        .with_extra_body(extra)
                ),
                Some(OptionsError::ReservedExtraBodyMember {
                    member: member.clone()
                }),
                "{member}"
            );
        }
    }
}
