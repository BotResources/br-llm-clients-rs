use std::task::Poll;

use br_llm_clients_core::{Client, ClientError};
use br_llm_messages::{ContentBlock, DeltaOp, ModelResponse};

use crate::MockClient;
use crate::fold::fold_deltas;
use crate::gate::Gate;
use crate::identity_support::*;
use crate::maximal::{Maximal, maximal};
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::sequential_id_source::{SequentialIdSource, identity_uuid};
use crate::test_support::*;

const START: u64 = 100;

fn client_with(source: &SequentialIdSource) -> MockClient {
    MockClient::builder(settings(), everything_caps())
        .id_source(source.clone())
        .build()
}

fn served(maximal: &Maximal) -> (ModelResponse, RecordingSink, SequentialIdSource) {
    let source = SequentialIdSource::starting_at(START);
    let client = client_with(&source);
    client.push_reply(Reply::success(maximal.response.clone()).build().unwrap());
    let (outcome, sink) = run(&client, &request());
    (response_from(&outcome), sink, source)
}

#[test]
fn given_a_response_with_every_kind_of_block_when_served_then_each_identity_is_the_next_of_the_source_in_the_order_of_the_response()
 {
    let built = maximal();
    let (given, _, source) = served(&built);
    let expected: Vec<_> = (0..built.draws as u64)
        .map(|k| identity_uuid(START + k))
        .collect();
    assert_eq!(ids_in_order(&given), expected);
    assert_eq!(source.given(), built.draws as u64);
}

#[test]
fn given_a_response_with_every_kind_of_block_when_served_then_nothing_but_the_identities_changes() {
    let built = maximal();
    let (given, _, _) = served(&built);
    assert_eq!(shape_of(&given), shape_of(&built.response));
    assert_ne!(ids_in_order(&given), ids_in_order(&built.response));
}

#[test]
fn given_a_response_when_served_then_the_entry_is_the_entry_of_the_request() {
    let built = maximal();
    let source = SequentialIdSource::starting_at(START);
    let client = client_with(&source);
    client.push_reply(Reply::success(built.response).build().unwrap());
    let request_value = request();
    let (outcome, sink) = run(&client, &request_value);
    assert_eq!(response_from(&outcome).id(), request_value.reply_entry());
    for delta in sink.deltas() {
        assert_eq!(delta.turn(), request_value.reply_turn());
        assert_eq!(delta.entry(), request_value.reply_entry());
    }
}

#[test]
fn given_a_provider_tool_result_when_served_then_it_names_the_new_identity_of_its_call() {
    let built = maximal();
    let (given, _, _) = served(&built);
    let call = given
        .blocks()
        .iter()
        .find_map(|block| match block {
            ContentBlock::ProviderToolCall(call) => Some(call.id()),
            _ => None,
        })
        .unwrap();
    assert_ne!(call, built.provider_call);
    let result = given
        .blocks()
        .iter()
        .find_map(|block| match block {
            ContentBlock::ProviderToolResult(result) => Some(result.call()),
            _ => None,
        })
        .unwrap();
    assert_eq!(result, call);
}

#[test]
fn given_a_provider_tool_result_that_names_no_call_of_the_response_when_served_then_the_reference_stays()
 {
    let foreign = crate::fixtures::block_id();
    let result = br_llm_messages::ProviderToolResult::new(
        crate::fixtures::block_id(),
        foreign,
        vec![crate::fixtures::text_block("found")],
    )
    .unwrap();
    let response = response_of(
        vec![ContentBlock::ProviderToolResult(result)],
        br_llm_messages::ResponseEnding::finished(),
        br_llm_messages::ResponseMetadata::new(),
    );
    let source = SequentialIdSource::starting_at(START);
    let client = client_with(&source);
    client.push_reply(Reply::success(response).build().unwrap());
    let (outcome, _) = run(&client, &request());
    let given = response_from(&outcome);
    let ContentBlock::ProviderToolResult(result) = &given.blocks()[0] else {
        panic!("result expected");
    };
    assert_eq!(result.call(), foreign);
}

#[test]
fn given_the_deltas_of_the_call_when_folded_then_they_give_the_response_the_call_gave() {
    let built = maximal();
    let (given, sink, _) = served(&built);
    assert_eq!(fold_deltas(&sink.deltas()).unwrap(), given);
}

#[test]
fn given_one_reply_served_twice_when_called_then_the_second_call_gets_new_identities() {
    let built = maximal();
    let source = SequentialIdSource::starting_at(START);
    let client = client_with(&source);
    client.set_fallback(Reply::success(built.response).build().unwrap());
    let first = response_from(&run(&client, &request()).0);
    let second = response_from(&run(&client, &request()).0);
    let overlap = ids_in_order(&first)
        .into_iter()
        .any(|id| ids_in_order(&second).contains(&id));
    assert!(!overlap);
    assert_eq!(shape_of(&first), shape_of(&second));
}

#[test]
fn given_a_gate_at_zero_when_the_call_waits_then_the_identities_are_already_taken_and_no_delta_is_sent()
 {
    let gate = Gate::new();
    let built = maximal();
    let source = SequentialIdSource::starting_at(START);
    let client = client_with(&source);
    client.push_reply(
        Reply::success(built.response)
            .gate_at(0, &gate)
            .build()
            .unwrap(),
    );
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    assert!(poll_once(&mut call.as_mut()).is_pending());
    assert_eq!(source.given(), built.draws as u64);
    drop(call);
    assert!(sink.is_empty());
    assert_eq!(source.given(), built.draws as u64);
}

#[test]
fn given_a_hang_when_called_then_the_identities_are_taken_before_the_first_delta() {
    let built = maximal();
    let source = SequentialIdSource::starting_at(START);
    let client = client_with(&source);
    client.push_reply(Reply::hang(built.response, 0).build().unwrap());
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = Box::pin(client.call(&request_value, &mut sink));
    assert!(matches!(poll_once(&mut call.as_mut()), Poll::Pending));
    assert_eq!(source.given(), built.draws as u64);
}

#[test]
fn given_replies_without_a_response_or_without_a_call_to_serve_when_called_then_no_identity_is_taken()
 {
    let source = SequentialIdSource::starting_at(START);
    let client = client_with(&source);
    client.push_reply(Reply::failure(ClientError::Transport).build().unwrap());
    client.push_reply(
        Reply::broken_stream(vec![started_delta()], ClientError::Transport)
            .build()
            .unwrap(),
    );
    run(&client, &request());
    run(&client, &request());
    run(&client, &request());
    assert_eq!(source.given(), 0);
}

#[test]
fn given_a_success_when_called_then_a_delta_kind_per_block_follows_the_order_of_the_identities() {
    let built = maximal();
    let (_, sink, _) = served(&built);
    let started = sink
        .deltas()
        .iter()
        .filter(|delta| {
            matches!(
                delta.op(),
                DeltaOp::BlockStarted { .. } | DeltaOp::BlockInserted { .. }
            )
        })
        .count();
    assert_eq!(started, built.response.blocks().len());
}

#[test]
fn given_a_structured_value_with_a_time_when_served_then_only_its_identity_changes() {
    let value = br_llm_messages::StructuredBlock::new(
        crate::fixtures::block_id(),
        br_llm_messages::OpaqueJson::parse(r#"{"a":[1,2]}"#).unwrap(),
    )
    .unwrap()
    .with_observed_at(br_llm_messages::ObservedAt::new("2026-10-27T10:00:00Z").unwrap());
    let response = response_of(
        vec![ContentBlock::StructuredContent(value)],
        br_llm_messages::ResponseEnding::finished(),
        br_llm_messages::ResponseMetadata::new(),
    );
    let source = SequentialIdSource::starting_at(START);
    let client = client_with(&source);
    client.push_reply(Reply::success(response.clone()).build().unwrap());
    let (outcome, _) = run(&client, &crate::response_support::structured_request());
    let given = response_from(&outcome);
    assert_eq!(shape_of(&given), shape_of(&response));
    assert_eq!(ids_in_order(&given), [identity_uuid(START)]);
}

#[test]
fn given_tool_arguments_and_a_structured_value_with_the_keys_id_and_call_when_served_then_the_keys_are_kept()
 {
    let json = r#"{"id":"mine","call":{"id":1},"list":[{"id":2,"call":3}]}"#;
    let kept = serde_json::from_str::<serde_json::Value>(json).unwrap();
    let cases = [
        (
            response_of(
                vec![call_content("search", json)],
                br_llm_messages::ResponseEnding::tool_use(),
                br_llm_messages::ResponseMetadata::new(),
            ),
            request(),
            "arguments",
        ),
        (
            response_of(
                vec![ContentBlock::StructuredContent(
                    br_llm_messages::StructuredBlock::new(
                        crate::fixtures::block_id(),
                        br_llm_messages::OpaqueJson::parse(json).unwrap(),
                    )
                    .unwrap(),
                )],
                br_llm_messages::ResponseEnding::finished(),
                br_llm_messages::ResponseMetadata::new(),
            ),
            crate::response_support::structured_request(),
            "value",
        ),
    ];
    for (response, request_value, field) in cases {
        let source = SequentialIdSource::starting_at(START);
        let client = client_with(&source);
        client.push_reply(Reply::success(response.clone()).build().unwrap());
        let (outcome, _) = run(&client, &request_value);
        let given = response_from(&outcome);
        assert_ne!(ids_in_order(&given), ids_in_order(&response));
        assert_eq!(shape_of(&given), shape_of(&response));
        let shape = shape_of(&given);
        assert_eq!(shape["content"][0][field], kept);
        let served = serde_json::to_value(&given).unwrap();
        assert_eq!(served["content"][0][field], kept);
    }
}
