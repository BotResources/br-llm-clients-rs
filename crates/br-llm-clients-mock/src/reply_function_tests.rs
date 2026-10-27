use std::time::Duration;

use br_llm_clients_core::ClientError;
use br_llm_messages::{ContentBlock, DeltaOp};

use crate::call_log::Served;
use crate::codes::CODE_INVALID_REPLY;
use crate::reply::Reply;
use crate::reply_source::ReplySource;
use crate::rule::Rule;
use crate::test_support::*;

fn call_ids(outcome: &br_llm_clients_core::CallOutcome) -> Vec<String> {
    response_from(outcome)
        .blocks()
        .iter()
        .filter_map(|block| match block {
            ContentBlock::RuntimeToolCall(call) => Some(call.id().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn given_a_value_reply_with_tool_calls_when_reused_then_each_call_gets_new_distinct_block_identities()
 {
    let client = mock();
    client.set_fallback(Reply::success(calls_response(2)).build().unwrap());
    let (first, first_sink) = run(&client, &request());
    let (second, _) = run(&client, &request());
    let (first_ids, second_ids) = (call_ids(&first), call_ids(&second));
    assert_eq!(first_ids.len(), 2);
    assert!(first_ids.iter().all(|id| !second_ids.contains(id)));
    let announced: Vec<String> = first_sink
        .deltas()
        .iter()
        .filter_map(|delta| match delta.op() {
            DeltaOp::BlockStarted { block, .. } => Some(block.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(announced, first_ids);
}

#[test]
fn given_a_function_reply_when_called_then_it_gets_request_settings_sequence_and_prior_claims() {
    let client = mock();
    client.set_fallback(ReplySource::function(|call| {
        let label = format!(
            "{}-{}-{}",
            call.settings().model().as_str(),
            call.sequence(),
            call.claimed_before()
        );
        Reply::text(label).build()
    }));
    let (first, _) = run(&client, &request());
    let (second, _) = run(&client, &request());
    let texts: Vec<String> = [first, second].iter().map(text_of).collect();
    assert_eq!(texts, ["settings-model-0-0", "settings-model-1-1"]);
}

#[test]
fn given_a_function_reply_when_called_then_it_reads_the_request_it_borrows() {
    let client = mock();
    client.set_fallback(ReplySource::function(|call| {
        Reply::text(format!("{}", call.request().tools().len())).build()
    }));
    let with_tool = request_of(
        vec![crate::fixtures::text_block("go")],
        vec![tool()],
        br_llm_clients_core::OutputMode::Plain,
    );
    let (outcome, _) = run(&client, &with_tool);
    assert_eq!(text_of(&outcome), "1");
}

#[test]
fn given_a_function_that_returns_a_build_error_when_called_then_provider_rejected_invalid_reply_names_the_origin()
 {
    let client = mock();
    client.push_reply(ReplySource::function(|_| {
        Reply::cut(text_response("x"), 99, ClientError::Transport).build()
    }));
    client.push_rule(Rule::new(
        |_| false,
        Reply::failure(ClientError::Transport).build().unwrap(),
    ));
    let (outcome, sink) = run(&client, &request());
    let ClientError::ProviderRejected { status, code, body } = error_of(&outcome) else {
        panic!("expected provider rejected");
    };
    assert_eq!(status, None);
    assert_eq!(code.as_deref(), Some(CODE_INVALID_REPLY));
    assert!(body.contains("script entry 0"));
    assert!(sink.is_empty());
    assert_eq!(outcome.report().total_duration(), Duration::ZERO);
    assert_eq!(outcome.report().usage(), None);
    assert_eq!(client.log()[0].served(), &Served::Entry { position: 0 });
}

#[test]
fn given_a_function_that_returns_a_build_error_when_called_then_it_takes_no_identity() {
    let source = crate::sequential_id_source::SequentialIdSource::new();
    let client = crate::mock_client::MockClient::builder(settings(), everything_caps())
        .id_source(source.clone())
        .build();
    client.set_fallback(ReplySource::function(|_| {
        Reply::hang(text_response("x"), 99).build()
    }));
    let _ = run(&client, &request());
    assert_eq!(source.given(), 0);
}

#[test]
fn given_invalid_replies_from_a_rule_and_the_fallback_when_called_then_the_body_names_them_with_position()
 {
    let bad = || ReplySource::function(|_| Reply::hang(text_response("x"), 99).build());
    let client = mock();
    client.push_rule(Rule::new(|_| false, bad()));
    client.push_rule(always_rule(bad()).with_limit(1).unwrap());
    client.set_fallback(bad());
    let (from_rule, _) = run(&client, &request());
    let (from_fallback, _) = run(&client, &request());
    let body = |outcome: &br_llm_clients_core::CallOutcome| match error_of(outcome) {
        ClientError::ProviderRejected { body, .. } => body,
        other => panic!("unexpected {other:?}"),
    };
    assert!(body(&from_rule).contains("rule 1"));
    assert!(body(&from_fallback).contains("fallback"));
}
