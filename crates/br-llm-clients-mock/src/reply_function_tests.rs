use std::time::Duration;

use br_llm_clients_core::{ClientError, OutputMode};
use br_llm_messages::StreamEvent;

use crate::call_log::Served;
use crate::codes::CODE_INVALID_REPLY;
use crate::reply::Reply;
use crate::rule::Rule;
use crate::test_support::*;

#[test]
fn given_a_value_reply_with_tool_calls_when_reused_in_one_conversation_then_the_second_use_is_malformed()
 {
    let client = mock();
    let reply = Reply::success(calls_step(&["fixed"])).build().unwrap();
    client.set_fallback(reply);
    let (first, _) = run(&client, &request());
    assert!(first.result().is_ok());
    let history = request_of(
        vec![
            user_text("go"),
            assistant_call("fixed"),
            user_result("fixed"),
        ],
        vec![tool()],
        OutputMode::Plain,
    );
    let (second, _) = run(&client, &history);
    assert!(matches!(
        error_of(&second),
        ClientError::MalformedResponse { .. }
    ));
}

#[test]
fn given_a_reply_that_renames_when_reused_in_one_conversation_then_every_call_is_accepted_with_distinct_ids()
 {
    let client = mock();
    client.set_fallback(
        Reply::success(calls_step(&["fixed", "other"]))
            .rename_tool_call_ids("call")
            .build()
            .unwrap(),
    );
    let (first, first_sink) = run(&client, &request());
    let first_ids: Vec<String> = step_of(&first)
        .tool_calls()
        .map(|call| call.id.to_string())
        .collect();
    assert_eq!(first_ids, ["call-0-0", "call-0-1"]);
    let history = request_of(
        vec![
            user_text("go"),
            assistant_call("call-0-0"),
            user_result("call-0-0"),
        ],
        vec![tool()],
        OutputMode::Plain,
    );
    let (second, _) = run(&client, &history);
    let second_ids: Vec<String> = step_of(&second)
        .tool_calls()
        .map(|call| call.id.to_string())
        .collect();
    assert_eq!(second_ids, ["call-1-0", "call-1-1"]);
    let announced: Vec<String> = first_sink
        .events()
        .iter()
        .filter_map(|event| match event {
            StreamEvent::ToolCallStart { id, .. } => Some(id.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(announced, first_ids);
}

#[test]
fn given_a_renamed_reply_whose_new_id_is_in_the_request_when_called_then_malformed_after_renaming()
{
    let client = mock();
    client.push_reply(
        Reply::success(calls_step(&["x"]))
            .rename_tool_call_ids("call")
            .build()
            .unwrap(),
    );
    let history = request_of(
        vec![
            user_text("go"),
            assistant_call("call-0-0"),
            user_result("call-0-0"),
        ],
        vec![tool()],
        OutputMode::Plain,
    );
    let (outcome, _) = run(&client, &history);
    assert!(matches!(
        error_of(&outcome),
        ClientError::MalformedResponse { .. }
    ));
}

#[test]
fn given_a_function_reply_when_called_then_it_gets_request_settings_sequence_and_prior_claims() {
    let client = mock();
    client.set_fallback(crate::reply_source::ReplySource::function(|call| {
        let label = format!(
            "{}-{}-{}-{}",
            call.request().messages().len(),
            call.settings().model(),
            call.sequence(),
            call.claimed_before()
        );
        Reply::success(text_step(&label)).build()
    }));
    let (first, _) = run(&client, &request());
    let (second, _) = run(&client, &request());
    let texts: Vec<String> = [first, second]
        .iter()
        .map(|outcome| step_of(outcome).text().next().unwrap().to_string())
        .collect();
    assert_eq!(texts, ["1-settings-model-0-0", "1-settings-model-1-1"]);
}

#[test]
fn given_a_function_that_returns_a_build_error_when_called_then_provider_rejected_invalid_reply_names_the_origin()
 {
    let client = mock();
    client.push_reply(crate::reply_source::ReplySource::function(|_| {
        Reply::cut(text_step("x"), 99, ClientError::Transport).build()
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
fn given_invalid_replies_from_a_rule_and_the_fallback_when_called_then_the_body_names_them_with_position()
 {
    let bad =
        || crate::reply_source::ReplySource::function(|_| Reply::hang(text_step("x"), 99).build());
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
