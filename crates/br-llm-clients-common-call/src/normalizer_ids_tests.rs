use br_llm_messages::StreamEvent;

use crate::generated_id::is_generated_id;
use crate::test_normalize::{feed, fresh, fresh_for, is_malformed, refusal};
use crate::test_requests::request_with_calls;

fn call_ids(events: &[StreamEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::ToolCallStart { id, .. } => Some(id.as_str().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn given_an_id_from_the_provider_when_started_then_it_is_kept_exactly() {
    let events = feed(&mut fresh(), &["start a tool search Call-Id.7:x"]);
    assert_eq!(call_ids(&events), ["Call-Id.7:x"]);
}

#[test]
fn given_no_id_or_an_empty_id_when_started_then_the_id_starts_with_the_generated_prefix() {
    let events = feed(
        &mut fresh(),
        &["start a tool search", "end a", "start b tool search ~"],
    );
    let ids = call_ids(&events);
    assert_eq!(ids.len(), 2);
    assert!(ids.iter().all(|id| is_generated_id(id)), "{ids:?}");
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn given_many_calls_without_an_id_when_started_then_every_generated_id_is_unique() {
    let mut normalizer = fresh();
    let mut ids = Vec::new();
    for round in 0..40 {
        let key = format!("k{round}");
        let start = format!("start {key} tool search");
        let end = format!("end {key}");
        ids.extend(call_ids(&feed(
            &mut normalizer,
            &[start.as_str(), end.as_str()],
        )));
    }
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 40);
}

#[test]
fn given_generated_ids_already_in_the_request_when_started_then_a_new_id_differs_from_all_of_them()
{
    let request = request_with_calls(&["brgen_0", "brgen_1", "call_x"]);
    let events = feed(
        &mut fresh_for(&request),
        &["start a tool search", "end a", "start b tool search"],
    );
    let ids = call_ids(&events);
    for id in &ids {
        assert!(is_generated_id(id));
        assert!(
            !["brgen_0", "brgen_1", "call_x"].contains(&id.as_str()),
            "{id}"
        );
    }
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn given_a_provider_id_with_the_generated_prefix_when_started_then_the_reply_is_malformed() {
    for id in ["brgen_1", "brgen_", "brgen_anything"] {
        let line = format!("start a tool search {id}");
        assert!(is_malformed(&refusal(&[], &line)), "{id}");
    }
}

#[test]
fn given_a_provider_id_that_is_not_quite_the_prefix_when_started_then_it_is_kept() {
    for id in ["brgen", "Brgen_1", "xbrgen_1", "brgen-1"] {
        let line = format!("start a tool search {id}");
        let events = feed(&mut fresh(), &[line.as_str()]);
        assert_eq!(call_ids(&events), [id]);
    }
}

#[test]
fn given_a_provider_id_used_twice_in_the_reply_when_started_then_the_reply_is_malformed() {
    let error = refusal(
        &["start a tool search dup", "end a"],
        "start b tool search dup",
    );
    assert!(is_malformed(&error), "{error:?}");
}

#[test]
fn given_a_provider_id_used_in_the_request_when_started_then_the_reply_is_malformed() {
    let request = request_with_calls(&["call_1"]);
    let mut normalizer = fresh_for(&request);
    let event = crate::test_decoder::script(&["start a tool search call_1"]).remove(0);
    let error = normalizer.accept(event).expect_err("refused");
    assert!(is_malformed(&error), "{error:?}");
    let event = crate::test_decoder::script(&["start a tool search call_2"]).remove(0);
    assert!(normalizer.accept(event).is_ok());
}

#[test]
fn given_the_prefix_when_asked_then_only_ids_that_start_with_it_are_generated() {
    assert!(is_generated_id("brgen_"));
    assert!(is_generated_id("brgen_0"));
    assert!(!is_generated_id("brgen"));
    assert!(!is_generated_id(" brgen_0"));
    assert!(!is_generated_id("call_brgen_0"));
    assert!(!is_generated_id(""));
}
