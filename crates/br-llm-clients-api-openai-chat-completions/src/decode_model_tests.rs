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
