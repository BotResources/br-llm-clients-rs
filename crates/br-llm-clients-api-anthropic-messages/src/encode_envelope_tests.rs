use br_llm_clients_common_call::Dialect;
use br_llm_clients_core::{CacheRequirement, OutputMode, ReasoningLevel};
use serde_json::{Value, json};

use crate::test_fixtures::*;
use crate::test_shapes::every_request_shape;

const ALLOWED_FIELDS: [&str; 9] = [
    "model",
    "max_tokens",
    "stream",
    "messages",
    "system",
    "tools",
    "thinking",
    "output_config",
    "cache_control",
];

const NEVER_SENT: [&str; 10] = [
    "tool_choice",
    "temperature",
    "top_p",
    "top_k",
    "stop_sequences",
    "metadata",
    "container",
    "fallbacks",
    "service_tier",
    "betas",
];

#[test]
fn given_every_request_shape_when_encoded_then_the_body_holds_only_the_closed_field_list() {
    for level in [ReasoningLevel::Off, ReasoningLevel::Max] {
        for cache in [CacheRequirement::Required, CacheRequirement::NotRequired] {
            let dialect = dialect_with(level, cache);
            for shape in every_request_shape() {
                let body = body_of(&dialect, &shape);
                for key in body.as_object().expect("object").keys() {
                    assert!(
                        ALLOWED_FIELDS.contains(&key.as_str()),
                        "{key} is not allowed"
                    );
                }
                for key in ["model", "max_tokens", "stream", "messages", "thinking"] {
                    assert!(body.get(key).is_some(), "{key} is missing");
                }
            }
        }
    }
}

#[test]
fn given_every_request_shape_when_encoded_then_no_forbidden_field_is_sent() {
    for shape in every_request_shape() {
        let text = String::from_utf8(dialect().encode(&shape).expect("encoded").body().to_vec())
            .expect("utf8");
        for key in NEVER_SENT {
            assert!(!text.contains(&format!("\"{key}\"")), "{key} was sent");
        }
        assert!(!text.contains("\"display\""));
        assert!(!text.contains("\"strict\""));
        assert!(!text.contains("budget_tokens"));
    }
}

#[test]
fn given_a_request_when_encoded_then_the_headers_are_exactly_content_type_and_version() {
    let encoded = dialect()
        .encode(&request(vec![user("hello")]))
        .expect("encoded");
    assert_eq!(
        encoded.headers(),
        &[
            ("content-type".to_owned(), "application/json".to_owned()),
            ("anthropic-version".to_owned(), "2023-06-01".to_owned()),
        ]
    );
}

#[test]
fn given_every_request_shape_when_encoded_then_no_credential_or_beta_header_is_returned() {
    for shape in every_request_shape() {
        let encoded = dialect().encode(&shape).expect("encoded");
        for (name, _) in encoded.headers() {
            let name = name.to_ascii_lowercase();
            assert!(!["anthropic-beta", "x-api-key", "authorization"].contains(&name.as_str()));
        }
    }
}

#[test]
fn given_the_settings_when_encoded_then_the_model_and_max_tokens_and_stream_are_sent() {
    let body = simple_body();
    assert_eq!(body["model"], json!(MODEL));
    assert_eq!(body["max_tokens"], json!(1024));
    assert_eq!(body["stream"], json!(true));
    assert_eq!(body["messages"], expected_simple_messages());
}

#[test]
fn given_a_largest_max_tokens_when_encoded_then_it_is_sent_as_it_is() {
    let dialect = crate::AnthropicMessages::new(&settings(
        ReasoningLevel::Off,
        CacheRequirement::NotRequired,
        u32::MAX,
    ));
    let body = body_of(&dialect, &request(vec![user("hello")]));
    assert_eq!(body["max_tokens"], json!(u32::MAX));
}

#[test]
fn given_no_system_and_no_tool_when_encoded_then_both_fields_are_absent() {
    let body = simple_body();
    assert!(body.get("system").is_none());
    assert!(body.get("tools").is_none());
}

#[test]
fn given_a_system_prompt_when_encoded_then_it_is_one_string_kept_as_it_is() {
    let request = request_with(
        Some("  be brief\n\nand kind  "),
        vec![user("hello")],
        vec![],
        OutputMode::Plain,
    );
    let body = body_of(&dialect(), &request);
    assert_eq!(body["system"], json!("  be brief\n\nand kind  "));
}

#[test]
fn given_the_same_request_when_encoded_twice_then_the_bytes_are_identical_and_the_request_is_unchanged()
 {
    let request = every_request_shape().remove(4);
    let before = request.clone();
    let first = dialect().encode(&request).expect("first");
    let second = dialect().encode(&request).expect("second");
    assert_eq!(first, second);
    assert_eq!(request, before);
}

#[test]
fn given_a_body_when_read_then_it_is_compact_json_without_insignificant_whitespace() {
    let encoded = dialect()
        .encode(&request(vec![user("hello")]))
        .expect("encoded");
    let value: Value = serde_json::from_slice(encoded.body()).expect("json");
    assert_eq!(encoded.body(), value.to_string().as_bytes());
}
