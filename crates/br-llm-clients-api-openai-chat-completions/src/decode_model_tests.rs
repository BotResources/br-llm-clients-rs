use br_llm_clients_common_call::ProviderEvent;
use serde_json::json;

use crate::test_support::{chunk, feed, plain_decoder};

#[test]
fn given_the_model_of_the_chunks_when_decoded_then_it_is_given_only_when_it_changes() {
    let mut decoder = plain_decoder();
    let named = |model: &str| chunk(json!({"content": "x"}), None).replace("gpt-4o", model);
    let given = |data: String, decoder: &mut dyn br_llm_clients_common_call::PayloadDecoder| {
        feed(decoder, &data)
            .expect("chunk")
            .into_iter()
            .filter_map(|event| match event {
                ProviderEvent::Model(model) => Some(model),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        given(named("gpt-4o-2024"), decoder.as_mut()),
        ["gpt-4o-2024"]
    );
    assert_eq!(
        given(named("gpt-4o-2024"), decoder.as_mut()),
        Vec::<String>::new()
    );
    assert_eq!(given(named(""), decoder.as_mut()), Vec::<String>::new());
    assert_eq!(given(named("gpt-5"), decoder.as_mut()), ["gpt-5"]);
    assert_eq!(
        given(named("gpt-4o-2024"), decoder.as_mut()),
        ["gpt-4o-2024"]
    );
}

#[test]
fn given_a_chunk_without_a_model_when_decoded_then_no_model_event_is_given() {
    let mut decoder = plain_decoder();
    let data = json!({"choices": [{"index": 0, "delta": {"content": "x"}}]}).to_string();
    let events = feed(decoder.as_mut(), &data).expect("chunk");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ProviderEvent::Model(_)))
    );
}

fn response_ids(
    decoder: &mut dyn br_llm_clients_common_call::PayloadDecoder,
    data: &str,
) -> Vec<String> {
    feed(decoder, data)
        .expect("chunk")
        .into_iter()
        .filter_map(|event| match event {
            ProviderEvent::ResponseId(id) => Some(id),
            _ => None,
        })
        .collect()
}

#[test]
fn given_the_id_of_the_chunks_when_decoded_then_a_response_id_is_given_only_when_it_changes() {
    let mut decoder = plain_decoder();
    let with_id = |id: &str| chunk(json!({"content": "x"}), None).replace("chatcmpl-123", id);
    assert_eq!(
        response_ids(decoder.as_mut(), &with_id("chatcmpl-1")),
        ["chatcmpl-1"]
    );
    assert!(response_ids(decoder.as_mut(), &with_id("chatcmpl-1")).is_empty());
    assert!(response_ids(decoder.as_mut(), &with_id("")).is_empty());
    assert_eq!(
        response_ids(decoder.as_mut(), &with_id("chatcmpl-2")),
        ["chatcmpl-2"]
    );
    assert_eq!(
        response_ids(decoder.as_mut(), &with_id("chatcmpl-1")),
        ["chatcmpl-1"]
    );
}

#[test]
fn given_a_chunk_without_an_id_or_with_a_null_id_when_decoded_then_no_response_id_is_given() {
    let mut decoder = plain_decoder();
    for data in [
        json!({"choices": [{"delta": {"content": "x"}}]}).to_string(),
        json!({"id": null, "choices": [{"delta": {"content": "x"}}]}).to_string(),
    ] {
        assert!(response_ids(decoder.as_mut(), &data).is_empty(), "{data}");
    }
}

#[test]
fn given_an_id_or_a_model_that_is_not_a_string_when_decoded_then_the_field_is_named() {
    for (field, value) in [("id", json!(7)), ("id", json!({})), ("model", json!(["m"]))] {
        let mut data = json!({"choices": []});
        data[field] = value.clone();
        let error = crate::test_support::fails(&data.to_string());
        assert!(
            error.text().contains(&format!("`{field}`")),
            "{field} {value}"
        );
    }
}

#[test]
fn given_a_usage_chunk_with_the_same_id_when_decoded_then_the_id_is_not_given_again() {
    let mut decoder = plain_decoder();
    let first = response_ids(decoder.as_mut(), &crate::test_support::content_chunk("a"));
    let usage = crate::test_support::usage_chunk(json!({"prompt_tokens": 1}));
    assert_eq!(first, ["chatcmpl-123"]);
    assert!(response_ids(decoder.as_mut(), &usage).is_empty());
}
