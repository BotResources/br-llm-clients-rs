use br_llm_clients_core::ClientError;

use crate::call_log::Served;
use crate::codes::CODE_SCRIPT_EXHAUSTED;
use crate::mock_client::MockClient;
use crate::reply::Reply;
use crate::test_support::*;

fn says(value: &str) -> Reply {
    Reply::success(text_response(value)).build().unwrap()
}

fn fails() -> Reply {
    Reply::failure(ClientError::Transport).build().unwrap()
}

fn answer_of(client: &MockClient) -> String {
    let (outcome, _) = run(client, &request());
    match outcome.result() {
        Ok(_) => text_of(&outcome),
        Err(error) => format!("{error:?}"),
    }
}

#[test]
fn given_a_script_when_calls_are_made_in_order_then_entries_are_used_in_order() {
    let client = MockClient::builder(settings(), everything_caps())
        .reply(says("one"))
        .reply(says("two"))
        .reply(says("three"))
        .build();
    assert_eq!(
        [answer_of(&client), answer_of(&client), answer_of(&client)],
        ["one", "two", "three"]
    );
    assert_eq!(client.unused_entries(), 0);
}

#[test]
fn given_an_exhausted_script_without_fallback_when_called_then_provider_rejected_with_the_exhausted_code()
 {
    let client = mock();
    let (outcome, sink) = run(&client, &request());
    let ClientError::ProviderRejected { status, code, body } = error_of(&outcome) else {
        panic!("expected provider rejected");
    };
    assert_eq!(status, None);
    assert_eq!(code.as_deref(), Some(CODE_SCRIPT_EXHAUSTED));
    assert!(!body.is_empty());
    assert!(sink.is_empty());
    assert_eq!(outcome.report().total_duration(), std::time::Duration::ZERO);
    assert_eq!(client.log()[0].served(), &Served::Exhausted);
}

#[test]
fn given_a_fallback_when_the_script_is_exhausted_then_the_fallback_replaces_the_error_without_limit()
 {
    let client = mock();
    client.push_reply(says("entry"));
    client.set_fallback(says("fallback"));
    let answers: Vec<String> = (0..4).map(|_| answer_of(&client)).collect();
    assert_eq!(answers, ["entry", "fallback", "fallback", "fallback"]);
    assert_eq!(client.log()[2].served(), &Served::Fallback);
}

#[test]
fn given_a_fallback_when_set_replaced_and_removed_then_the_next_call_sees_the_change_from_any_handle()
 {
    let client = mock();
    let other_handle = client.clone();
    client.set_fallback(says("a"));
    assert_eq!(answer_of(&other_handle), "a");
    other_handle.set_fallback(says("b"));
    assert_eq!(answer_of(&client), "b");
    client.remove_fallback();
    assert!(answer_of(&other_handle).contains("Provider"));
}

#[test]
fn given_replies_added_after_build_when_called_then_they_go_at_the_end() {
    let client = MockClient::builder(settings(), everything_caps())
        .reply(says("first"))
        .build();
    client.push_reply(says("second"));
    assert_eq!(
        [answer_of(&client), answer_of(&client)],
        ["first", "second"]
    );
}

#[test]
fn given_an_entry_added_before_the_claim_when_a_call_claims_then_the_entry_is_seen() {
    let client = mock();
    client.push_reply(says("early"));
    client.push_reply(says("later"));
    assert_eq!([answer_of(&client), answer_of(&client)], ["early", "later"]);
}

#[test]
fn given_calls_polled_in_order_and_finishing_in_reverse_when_claiming_then_the_claim_order_is_the_poll_order()
 {
    let gate = crate::Gate::new();
    let client = mock();
    for label in ["first", "second"] {
        let reply = Reply::success(text_response(label))
            .gate_at(0, &gate)
            .build()
            .unwrap();
        client.push_reply(reply);
    }
    let request_value = request();
    let mut sink_a = crate::RecordingSink::new();
    let mut sink_b = crate::RecordingSink::new();
    let mut call_a = Box::pin(br_llm_clients_core::Client::call(
        &client,
        &request_value,
        &mut sink_a,
    ));
    let mut call_b = Box::pin(br_llm_clients_core::Client::call(
        &client,
        &request_value,
        &mut sink_b,
    ));
    assert!(poll_once(&mut call_a.as_mut()).is_pending());
    assert!(poll_once(&mut call_b.as_mut()).is_pending());
    gate.open();
    let second = block_on(call_b.as_mut());
    let first = block_on(call_a.as_mut());
    assert_eq!(text_of(&second), "second");
    assert_eq!(text_of(&first), "first");
}

#[test]
fn given_an_unpolled_call_when_dropped_then_it_leaves_no_trace() {
    let client = mock();
    client.push_reply(says("kept"));
    let request_value = request();
    let mut sink = crate::RecordingSink::new();
    drop(br_llm_clients_core::Client::call(
        &client,
        &request_value,
        &mut sink,
    ));
    assert_eq!(client.calls(), 0);
    assert!(client.log().is_empty());
    assert_eq!(client.unused_entries(), 1);
    let _ = fails();
}
