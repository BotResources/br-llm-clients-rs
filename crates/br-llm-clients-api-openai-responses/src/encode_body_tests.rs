use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::{CacheRequirement, ClientRequest, OutputMode, ReasoningLevel};
use serde_json::{Value, json};

use crate::OpenAiResponses;
use crate::test_requests::{
    body_of, dialect, encode_body, hello, parameter, request_with, settings, tool, user_text,
};
use br_llm_clients_core::ParameterKind;

fn keys(body: &Value) -> Vec<String> {
    let mut keys: Vec<String> = body.as_object().expect("object").keys().cloned().collect();
    keys.sort();
    keys
}

#[test]
fn given_a_plain_request_when_encoded_then_the_body_holds_exactly_the_documented_fields() {
    let body = body_of(&hello());
    assert_eq!(
        keys(&body),
        [
            "input",
            "max_output_tokens",
            "model",
            "reasoning",
            "store",
            "stream"
        ]
    );
}

#[test]
fn given_a_request_when_encoded_then_stream_is_true_and_store_is_false() {
    let body = body_of(&hello());
    assert_eq!(body["stream"], json!(true));
    assert_eq!(body["store"], json!(false));
}

#[test]
fn given_the_settings_when_encoded_then_the_model_and_the_token_limit_pass_unchanged() {
    let body = body_of(&hello());
    assert_eq!(body["model"], json!("gpt-5.1"));
    assert_eq!(body["max_output_tokens"], json!(4096));
}

#[test]
fn given_a_system_prompt_when_encoded_then_it_is_the_instructions() {
    let request = request_with(
        Some("be brief"),
        vec![user_text("hi")],
        Vec::new(),
        OutputMode::Plain,
    );
    assert_eq!(body_of(&request)["instructions"], json!("be brief"));
}

#[test]
fn given_no_system_prompt_when_encoded_then_there_is_no_instructions_field() {
    assert!(body_of(&hello()).get("instructions").is_none());
}

#[test]
fn given_a_full_request_when_encoded_then_no_state_or_tuning_field_is_sent() {
    let request = request_with(
        Some("system"),
        vec![user_text("hi")],
        vec![tool("search", "", Vec::new())],
        OutputMode::Plain,
    );
    let dialect = crate::test_requests::dialect(ReasoningLevel::High);
    let body = encode_body(&dialect, &request).expect("encoded");
    for forbidden in [
        "previous_response_id",
        "truncation",
        "tool_choice",
        "parallel_tool_calls",
        "temperature",
        "top_p",
        "conversation",
        "background",
        "metadata",
        "prompt_cache_key",
    ] {
        assert!(body.get(forbidden).is_none(), "{forbidden}");
    }
}

#[test]
fn given_no_tools_when_encoded_then_there_is_no_tools_field() {
    assert!(body_of(&hello()).get("tools").is_none());
}

#[test]
fn given_a_plain_output_when_encoded_then_there_is_no_text_field() {
    assert!(body_of(&hello()).get("text").is_none());
}

#[test]
fn given_a_request_when_encoded_then_the_only_header_is_content_type_json() {
    let encoded = dialect(ReasoningLevel::Off)
        .encode(&hello())
        .expect("encoded");
    assert_eq!(
        encoded.headers(),
        [("Content-Type".to_owned(), "application/json".to_owned())]
    );
}

#[test]
fn given_either_cache_requirement_when_encoded_then_the_body_is_identical_and_no_error() {
    let encode_with = |cache| {
        let dialect = OpenAiResponses::new(
            &settings(ReasoningLevel::Medium, cache),
            crate::Options::default(),
        );
        dialect.encode(&hello()).expect("never CacheNotSupported")
    };
    assert_eq!(
        encode_with(CacheRequirement::Required),
        encode_with(CacheRequirement::NotRequired)
    );
}

#[test]
fn given_the_same_request_twice_when_encoded_then_the_bytes_are_identical() {
    let dialect = dialect(ReasoningLevel::High);
    let request: ClientRequest = hello();
    assert_eq!(
        dialect.encode(&request).expect("first"),
        dialect.encode(&request).expect("second")
    );
}

#[test]
fn given_a_declared_tool_when_encoded_then_tools_is_an_array_with_that_tool() {
    let request = request_with(
        None,
        vec![user_text("hi")],
        vec![tool(
            "search",
            "find",
            vec![parameter("q", "", true, ParameterKind::String)],
        )],
        OutputMode::Plain,
    );
    let body = body_of(&request);
    assert_eq!(body["tools"].as_array().expect("array").len(), 1);
}
