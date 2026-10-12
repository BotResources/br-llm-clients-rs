use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_core::{CacheRequirement, ReasoningLevel};

use crate::build_error::MockBuildError;
use crate::call_log::Served;
use crate::mock_client::MockClient;
use crate::reply::Reply;
use crate::reply_source::ReplySource;
use crate::rule::Rule;
use crate::test_support::*;

fn says(value: &str) -> Reply {
    Reply::success(text_step(value)).build().unwrap()
}

fn answer_of(client: &MockClient) -> String {
    let (outcome, _) = run(client, &request());
    match outcome.result() {
        Ok(step_value) => step_value.text().next().unwrap().to_string(),
        Err(error) => format!("{error:?}"),
    }
}

#[test]
fn given_a_matching_rule_when_called_then_the_rule_serves_and_the_script_is_untouched() {
    let client = mock();
    client.push_reply(says("entry"));
    client.push_rule(Rule::always(says("rule")));
    assert_eq!(answer_of(&client), "rule");
    assert_eq!(client.unused_entries(), 1);
    assert_eq!(client.log()[0].served(), &Served::Rule { position: 0 });
}

#[test]
fn given_rules_when_tried_then_the_first_matching_in_the_order_added_wins() {
    let client = mock();
    client.push_rule(Rule::new(|_| false, says("never")));
    client.push_rule(Rule::always(says("second")));
    client.push_rule(Rule::always(says("third")));
    assert_eq!(answer_of(&client), "second");
    assert_eq!(client.log()[0].served(), &Served::Rule { position: 1 });
}

#[test]
fn given_a_rule_with_a_limit_when_it_is_used_up_then_it_is_skipped_and_its_matcher_is_not_called() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let client = mock();
    client.push_rule(
        Rule::new(
            move |_| {
                counted.fetch_add(1, Ordering::SeqCst);
                true
            },
            says("limited"),
        )
        .with_limit(2)
        .unwrap(),
    );
    client.push_reply(says("entry"));
    let answers: Vec<String> = (0..3).map(|_| answer_of(&client)).collect();
    assert_eq!(answers, ["limited", "limited", "entry"]);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(client.unused_uses(), 0);
}

#[test]
fn given_a_limit_of_zero_when_a_rule_is_built_then_the_builder_returns_rule_limit_zero() {
    assert_eq!(
        Rule::always(says("x")).with_limit(0).unwrap_err(),
        MockBuildError::RuleLimitZero
    );
}

#[test]
fn given_rules_with_limits_when_counted_then_unused_uses_is_the_sum_of_what_is_left() {
    let client = mock();
    client.push_rule(Rule::always(says("a")).with_limit(3).unwrap());
    client.push_rule(Rule::new(|_| false, says("b")).with_limit(4).unwrap());
    client.push_rule(Rule::new(|_| false, says("unlimited")));
    assert_eq!(client.unused_uses(), 7);
    answer_of(&client);
    assert_eq!(client.unused_uses(), 6);
}

#[test]
fn given_a_rule_added_while_a_call_runs_its_matchers_when_the_call_claims_then_that_call_does_not_see_it()
 {
    let client = mock();
    let adder = client.clone();
    client.push_rule(Rule::new(
        move |_| {
            adder.push_rule(Rule::always(says("late rule")));
            false
        },
        says("never"),
    ));
    client.push_reply(says("entry"));
    assert_eq!(answer_of(&client), "entry");
    assert_eq!(answer_of(&client), "late rule");
}

#[test]
fn given_a_matcher_when_called_then_it_gets_request_settings_and_sequence() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink_of_seen = Arc::clone(&seen);
    let client = mock();
    client.push_rule(Rule::new(
        move |call| {
            sink_of_seen.lock().unwrap().push((
                call.request().messages().len(),
                call.settings().model().to_string(),
                call.sequence(),
            ));
            false
        },
        says("never"),
    ));
    client.set_fallback(says("fallback"));
    answer_of(&client);
    answer_of(&client);
    assert_eq!(
        *seen.lock().unwrap(),
        [
            (1, "settings-model".to_owned(), 0),
            (1, "settings-model".to_owned(), 1)
        ]
    );
}

#[test]
fn given_a_matcher_on_the_settings_when_handles_differ_then_each_handle_is_served_by_its_own_rule()
{
    let client = mock();
    client.push_rule(Rule::new(
        |call| call.settings().max_output_tokens().get() > 5000,
        says("big"),
    ));
    client.set_fallback(says("small"));
    let big = client.with_settings(settings_with(
        ReasoningLevel::Off,
        CacheRequirement::NotRequired,
        9000,
    ));
    assert_eq!(answer_of(&client), "small");
    assert_eq!(answer_of(&big), "big");
}

#[test]
fn given_a_handle_with_other_settings_when_used_then_it_shares_script_rules_fallback_and_log() {
    let client = mock();
    let other = client.with_settings(settings_with(
        ReasoningLevel::High,
        CacheRequirement::Required,
        7,
    ));
    client.push_reply(says("one"));
    other.push_reply(says("two"));
    assert_eq!(answer_of(&other), "one");
    assert_eq!(answer_of(&client), "two");
    assert_eq!(client.calls(), 2);
    let log = other.log();
    assert_eq!(log[0].settings().max_output_tokens().get(), 7);
    assert_eq!(log[1].settings().max_output_tokens().get(), 1000);
    assert_eq!(other.capabilities(), client.capabilities());
}

#[test]
fn given_a_reply_function_when_claimed_by_a_rule_then_it_counts_the_uses_of_that_rule_only() {
    let client = mock();
    let counting = ReplySource::function(|call| {
        Reply::success(text_step(&format!("use {}", call.claimed_before()))).build()
    });
    client.push_rule(Rule::new(
        |call| call.request().messages().len() == 1,
        counting,
    ));
    client.push_rule(Rule::new(|_| false, says("unused")));
    assert_eq!([answer_of(&client), answer_of(&client)], ["use 0", "use 1"]);
}

#[test]
fn given_replacing_the_fallback_when_a_function_counts_then_the_count_of_the_fallback_is_not_reset()
{
    let counting = || {
        ReplySource::function(|call| {
            Reply::success(text_step(&format!("fallback {}", call.claimed_before()))).build()
        })
    };
    let client = mock();
    client.set_fallback(counting());
    answer_of(&client);
    answer_of(&client);
    client.set_fallback(counting());
    assert_eq!(answer_of(&client), "fallback 2");
}

#[test]
fn given_entries_and_rules_mixed_when_calls_are_made_then_the_log_names_how_each_was_served() {
    let client = mock();
    client.push_rule(Rule::new(
        |call| call.request().messages().len() > 1,
        says("rule"),
    ));
    client.push_reply(says("entry"));
    client.set_fallback(says("fallback"));
    let long = request_of(
        vec![user_text("a"), assistant_text("b"), user_text("c")],
        vec![],
        br_llm_clients_core::OutputMode::Plain,
    );
    run(&client, &request());
    run(&client, &long);
    run(&client, &request());
    let served: Vec<Served> = client
        .log()
        .iter()
        .map(|call| call.served().clone())
        .collect();
    assert_eq!(
        served,
        [
            Served::Entry { position: 0 },
            Served::Rule { position: 0 },
            Served::Fallback
        ]
    );
}
