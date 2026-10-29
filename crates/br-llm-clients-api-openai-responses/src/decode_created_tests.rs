use br_llm_clients_common_call::ProviderEvent;
use serde_json::json;

use crate::test_decoder::{feed, feed_err, model, plain_decoder};
use crate::test_events::{completed, created, usage_object};

#[test]
fn given_an_empty_or_absent_response_id_when_created_then_no_response_id_event_comes() {
    let mut event = created("m");
    event["response"]["id"] = json!("");
    assert_eq!(feed(&mut plain_decoder(), &event), vec![model("m")]);
    event["response"]["id"] = json!(null);
    assert_eq!(feed(&mut plain_decoder(), &event), vec![model("m")]);
    event["response"]
        .as_object_mut()
        .expect("response")
        .remove("id");
    assert_eq!(feed(&mut plain_decoder(), &event), vec![model("m")]);
}

#[test]
fn given_a_response_id_that_is_not_a_text_when_created_then_it_is_a_payload_error() {
    let mut event = created("m");
    event["response"]["id"] = json!(42);
    let error = feed_err(&mut plain_decoder(), &event);
    assert!(error.text().starts_with("response.created:"), "{error}");
}

#[test]
fn given_a_terminal_event_with_a_response_id_when_finished_then_no_response_id_event_comes() {
    let events = feed(&mut plain_decoder(), &completed("m", usage_object(1, 1)));
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, ProviderEvent::ResponseId(_)))
    );
}
