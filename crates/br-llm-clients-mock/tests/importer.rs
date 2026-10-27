mod support;

use std::sync::Arc;
use std::task::{Context, Waker};

use br_llm_clients_core::{
    CacheRequirement, Client, ClientError, ClientSettings, MaxOutputTokens, ReasoningLevel,
};
use br_llm_clients_mock::{
    Accepted, Capabilities, CapabilitiesSpec, Chunking, DEFAULT_ID_START, Gate, MockClient,
    RecordingSink, Reply, ReplySource, Rule, SequentialIdSource, Sketch, StateKind, derive_deltas,
    fold_deltas, fold_partial,
};
use br_llm_messages::{ContentBlock, ModelName, ResponseEnding};
use support::{block_on, everything, request, settings};

fn always(reply: Reply) -> Rule {
    Rule::new(|_| true, reply)
}

fn text_of(response: &br_llm_messages::ModelResponse) -> String {
    response
        .blocks()
        .iter()
        .find_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text().as_str().to_owned()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn given_the_public_surface_when_a_test_writes_a_script_then_the_code_under_test_gets_the_answers()
{
    let client = MockClient::builder(settings(), everything())
        .reply(Reply::text("scripted").build().unwrap())
        .build();
    let shared: Arc<dyn Client> = Arc::new(client.clone());
    let mut sink = RecordingSink::new();
    let outcome = block_on(shared.call(&request(), &mut sink));
    assert_eq!(text_of(outcome.result().unwrap()), "scripted");
    assert_eq!(client.calls(), 1);
    assert_eq!(client.log()[0].state().kind(), StateKind::Succeeded);
    assert_eq!(&sink.fold().unwrap(), outcome.result().unwrap());
}

#[test]
fn given_handles_of_one_mock_when_asked_for_their_settings_then_each_gives_back_its_own() {
    let original = MockClient::new(settings(), everything());
    let clone = original.clone();
    let wider = ClientSettings::new(
        ModelName::new("other-model").unwrap(),
        ReasoningLevel::High,
        CacheRequirement::Required,
        MaxOutputTokens::new(4096).unwrap(),
    );
    let retry = clone.with_settings(wider.clone());
    let back = retry.with_settings(settings());
    let handles: [&dyn Client; 5] = [&original, &clone, &retry, &retry.clone(), &back];
    assert_eq!(
        handles.map(|handle| handle.settings().clone()),
        [settings(), settings(), wider.clone(), wider, settings()]
    );
    original.set_fallback(Reply::text("any").build().unwrap());
    block_on(retry.call(&request(), &mut RecordingSink::new()));
    assert_eq!(original.log()[0].settings(), retry.settings());
}

#[test]
fn given_capabilities_built_from_everything_when_changed_then_strictness_refuses_like_a_real_client()
 {
    let spec = CapabilitiesSpec {
        reasoning_levels: Accepted::Only(vec![ReasoningLevel::Low]),
        ..CapabilitiesSpec::everything()
    };
    let client = MockClient::new(settings(), Capabilities::new(spec).unwrap());
    let mut sink = RecordingSink::new();
    let outcome = block_on(client.call(&request(), &mut sink));
    assert!(matches!(
        outcome.result(),
        Err(ClientError::ReasoningNotSupported { .. })
    ));
    assert!(sink.is_empty());
}

#[test]
fn given_a_reply_function_when_a_tool_call_is_scripted_then_the_identities_are_new_per_call_and_the_same_on_every_run()
 {
    let run = || {
        let client = MockClient::new(settings(), everything());
        client.set_fallback(ReplySource::function(|call| {
            Reply::tool_call("search", format!(r#"{{"call":{}}}"#, call.sequence())).build()
        }));
        (0..2)
            .map(|_| {
                let mut sink = RecordingSink::new();
                let outcome = block_on(client.call(&request(), &mut sink));
                outcome.result().unwrap().blocks()[0].id()
            })
            .collect::<Vec<_>>()
    };
    let first = run();
    assert_ne!(first[0], first[1]);
    assert_eq!(first, run());
}

#[test]
fn given_a_rule_and_a_gate_when_a_test_drives_a_call_by_hand_then_the_gate_decides_when_it_goes_on()
{
    let gate = Gate::new();
    let client = MockClient::new(settings(), everything());
    client.push_rule(always(
        Reply::text("held")
            .gate_at(1, &gate)
            .chunking(Chunking::none().text(2))
            .build()
            .unwrap(),
    ));
    let request_value = request();
    let mut sink = RecordingSink::new();
    let mut call = std::pin::pin!(client.call(&request_value, &mut sink));
    let mut context = Context::from_waker(Waker::noop());
    assert!(call.as_mut().poll(&mut context).is_pending());
    assert_eq!(gate.waiting(), 1);
    gate.open();
    assert!(call.as_mut().poll(&mut context).is_ready());
}

#[test]
fn given_a_response_when_deltas_are_derived_then_the_list_ends_with_completed_and_folds_back() {
    let response = Sketch::new().text("abc").to_response().unwrap();
    let request_value = request();
    let derived = derive_deltas(request_value.reply_turn(), &response, &Chunking::none()).unwrap();
    assert_eq!(derived.before_completed() + 1, derived.deltas().len());
    assert_eq!(fold_deltas(derived.deltas()).unwrap(), response);
    let cut = fold_partial(&derived.deltas()[..2]).unwrap();
    assert!(cut.stop().is_none());
}

#[test]
fn given_a_call_waiting_at_a_gate_when_another_thread_opens_it_then_the_parked_caller_goes_on() {
    let gate = Gate::new();
    let client = MockClient::new(settings(), everything());
    client.push_rule(always(
        Reply::text("released").gate_at(0, &gate).build().unwrap(),
    ));
    let opener = {
        let gate = gate.clone();
        std::thread::spawn(move || {
            block_on(gate.wait_for_waiting(1));
            gate.open();
        })
    };
    let mut sink = RecordingSink::new();
    let outcome = block_on(client.call(&request(), &mut sink));
    opener.join().unwrap();
    assert!(outcome.result().is_ok());
    assert!(gate.is_open());
}

#[test]
fn given_two_mocks_in_one_conversation_when_they_share_one_source_then_their_identities_never_meet()
{
    let source = SequentialIdSource::starting_at(DEFAULT_ID_START);
    let build = |text: &str| {
        let client = MockClient::builder(settings(), everything())
            .id_source(source.clone())
            .build();
        client.set_fallback(Reply::text(text).build().unwrap());
        client
    };
    let (first, second) = (build("a"), build("b"));
    let one = block_on(first.call(&request(), &mut RecordingSink::new()));
    let two = block_on(second.call(&request(), &mut RecordingSink::new()));
    assert_ne!(
        one.result().unwrap().blocks()[0].id(),
        two.result().unwrap().blocks()[0].id()
    );
    assert_eq!(source.given(), 2);
}

#[test]
fn given_a_sketch_with_a_tool_call_when_served_then_the_ending_is_tool_use() {
    let client = MockClient::new(settings(), everything());
    client.set_fallback(
        Reply::success(
            Sketch::new()
                .tool_call("search", "{}")
                .ending(ResponseEnding::finished()),
        )
        .build()
        .unwrap(),
    );
    let outcome = block_on(client.call(&request(), &mut RecordingSink::new()));
    assert_eq!(
        *outcome.result().unwrap().ending(),
        ResponseEnding::tool_use()
    );
}
