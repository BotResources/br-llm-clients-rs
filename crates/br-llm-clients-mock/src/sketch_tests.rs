use br_llm_clients_core::ClientError;
use br_llm_messages::{
    ContentBlock, ModelResponse, OpaqueJson, ReplayFormat, ReplayMaterial, ReplayPayload,
    ResponseEnding, Usage,
};

use crate::build_error::MockBuildError;
use crate::chunking::Chunking;
use crate::derive_deltas::derive_deltas;
use crate::reply::Reply;
use crate::sketch::Sketch;
use crate::test_support::*;

fn replay() -> ReplayMaterial {
    ReplayMaterial::new(
        ReplayFormat::new("fmt").unwrap(),
        ReplayPayload::text("opaque"),
    )
}

fn invalid(sketch: Sketch) -> bool {
    matches!(
        sketch.to_response(),
        Err(MockBuildError::SketchInvalid { reason }) if !reason.is_empty()
    )
}

#[test]
fn given_a_sketch_with_every_kind_when_converted_then_the_blocks_keep_their_order() {
    let response = Sketch::new()
        .thinking("hm")
        .text("one")
        .tool_call("search", r#"{"q":1}"#)
        .structured(r#"{"a":2}"#)
        .text("two")
        .to_response()
        .unwrap();
    let kinds: Vec<_> = response.blocks().iter().map(ContentBlock::kind).collect();
    assert_eq!(
        kinds,
        [
            br_llm_messages::ContentBlockKind::Thinking,
            br_llm_messages::ContentBlockKind::Text,
            br_llm_messages::ContentBlockKind::RuntimeToolCall,
            br_llm_messages::ContentBlockKind::StructuredContent,
            br_llm_messages::ContentBlockKind::Text,
        ]
    );
}

#[test]
fn given_a_sketch_without_ending_when_converted_then_tool_use_with_a_call_and_finished_without() {
    let with_call = Sketch::new()
        .tool_call("search", "{}")
        .to_response()
        .unwrap();
    assert_eq!(*with_call.ending(), ResponseEnding::tool_use());
    let without = Sketch::new().text("hi").to_response().unwrap();
    assert_eq!(*without.ending(), ResponseEnding::finished());
}

#[test]
fn given_a_sketch_with_an_ending_when_converted_then_the_ending_is_kept() {
    let response = Sketch::new()
        .text("cut")
        .ending(ResponseEnding::output_exhaustion())
        .to_response()
        .unwrap();
    assert_eq!(*response.ending(), ResponseEnding::output_exhaustion());
}

#[test]
fn given_a_sketch_with_metadata_when_converted_then_usage_model_and_provider_id_are_kept() {
    let response = Sketch::new()
        .text("hi")
        .usage(usage(3, 4))
        .model("answering")
        .provider_response_id("resp_1")
        .to_response()
        .unwrap();
    assert_eq!(response.usage(), Some(&usage(3, 4)));
    assert_eq!(response.model(), Some(&model("answering")));
    assert_eq!(response.provider_response_id().unwrap().as_str(), "resp_1");
    let bare = Sketch::new().text("hi").to_response().unwrap();
    assert_eq!(bare.usage(), None);
    assert_eq!(bare.model(), None);
    assert_eq!(bare.provider_response_id(), None);
}

#[test]
fn given_a_tool_call_with_an_external_id_and_thinking_with_replay_when_converted_then_they_are_kept()
 {
    let response = Sketch::new()
        .thinking_with_replay("hm", replay())
        .tool_call_with_external_id("search", "call_9", "{}")
        .to_response()
        .unwrap();
    let ContentBlock::Thinking(thinking) = &response.blocks()[0] else {
        panic!("thinking expected");
    };
    assert_eq!(thinking.replay(), Some(&replay()));
    let ContentBlock::RuntimeToolCall(call) = &response.blocks()[1] else {
        panic!("call expected");
    };
    assert_eq!(call.external_id().unwrap().as_str(), "call_9");
}

#[test]
fn given_a_thinking_with_empty_text_when_it_has_replay_then_it_is_accepted_and_without_replay_refused()
 {
    assert!(
        Sketch::new()
            .thinking_with_replay("", replay())
            .to_response()
            .is_ok()
    );
    assert!(invalid(Sketch::new().thinking("")));
}

#[test]
fn given_json_as_a_value_or_as_a_text_when_converted_then_both_give_the_same_response() {
    let value = OpaqueJson::parse(r#"{"a":[1,2]}"#).unwrap();
    let from_value = Sketch::new().structured(value).to_response().unwrap();
    let from_text = Sketch::new()
        .structured(r#"{"a":[1,2]}"#)
        .to_response()
        .unwrap();
    assert_eq!(from_value.blocks().len(), 1);
    assert_eq!(from_value, from_text);
}

#[test]
fn given_a_sketch_that_breaks_a_rule_of_messages_when_converted_then_sketch_invalid_with_the_reason()
 {
    assert!(invalid(Sketch::new().text("   ")));
    assert!(invalid(Sketch::new().text("")));
    assert!(invalid(Sketch::new().tool_call("", "{}")));
    assert!(invalid(Sketch::new().tool_call("search", "[1]")));
    assert!(invalid(Sketch::new().tool_call("search", "{")));
    assert!(invalid(Sketch::new().structured("{")));
    assert!(invalid(Sketch::new().text("x").model("")));
    assert!(invalid(Sketch::new().text("x").provider_response_id("")));
    assert!(invalid(
        Sketch::new().text("x").ending(ResponseEnding::tool_use())
    ));
}

#[test]
fn given_an_invalid_sketch_when_a_reply_is_built_then_sketch_invalid_whatever_the_kind() {
    let bad = || Sketch::new().text("");
    let is_sketch_invalid = |result: Result<Reply, MockBuildError>| {
        matches!(result, Err(MockBuildError::SketchInvalid { .. }))
    };
    assert!(is_sketch_invalid(Reply::success(bad()).build()));
    assert!(is_sketch_invalid(
        Reply::cut(bad(), 0, ClientError::Transport).build()
    ));
    assert!(is_sketch_invalid(Reply::hang(bad(), 0).build()));
    assert!(is_sketch_invalid(Reply::text("  ").build()));
    assert!(is_sketch_invalid(Reply::tool_call("search", "[]").build()));
    assert!(is_sketch_invalid(Reply::structured("{").build()));
}

#[test]
fn given_the_short_forms_when_built_then_each_holds_the_one_block_it_names() {
    let text = Reply::text("hello").build().unwrap();
    let call = Reply::tool_call("search", r#"{"q":1}"#).build().unwrap();
    let value = Reply::structured(r#"{"a":1}"#).build().unwrap();
    let kinds = |reply: &Reply| {
        reply
            .response()
            .unwrap()
            .blocks()
            .iter()
            .map(ContentBlock::kind)
            .collect::<Vec<_>>()
    };
    assert_eq!(kinds(&text), [br_llm_messages::ContentBlockKind::Text]);
    assert_eq!(
        kinds(&call),
        [br_llm_messages::ContentBlockKind::RuntimeToolCall]
    );
    assert_eq!(
        kinds(&value),
        [br_llm_messages::ContentBlockKind::StructuredContent]
    );
}

fn sketched_and_built() -> (Sketch, ModelResponse) {
    let sketch = Sketch::new()
        .text("alpha")
        .tool_call("search", r#"{"q":1}"#)
        .usage(usage(2, 3))
        .model("answering");
    let built = response_of(
        vec![text_content("alpha"), call_content("search", r#"{"q":1}"#)],
        ResponseEnding::tool_use(),
        br_llm_messages::ResponseMetadata::new()
            .with_usage(usage(2, 3))
            .with_model(model("answering")),
    );
    (sketch, built)
}

#[test]
fn given_a_sketch_and_a_response_of_the_same_content_when_served_then_the_outcome_is_the_same() {
    let (sketch, built) = sketched_and_built();
    let request_value = request();
    let serve = |reply: Reply| {
        let client = mock();
        client.push_reply(reply);
        let (outcome, sink) = run(&client, &request_value);
        (outcome, sink.deltas())
    };
    let from_sketch = serve(Reply::success(sketch).build().unwrap());
    let from_response = serve(Reply::success(built).build().unwrap());
    assert!(from_sketch.0.result().is_ok());
    assert_eq!(from_sketch, from_response);
}

#[test]
fn given_a_sketch_when_a_reply_is_checked_then_the_delta_count_is_the_one_of_the_response() {
    let (sketch, built) = sketched_and_built();
    let chunking = Chunking::none().text(2).tool_arguments(3);
    let turn = crate::fixtures::turn_id();
    let count = derive_deltas(turn, &sketch, &chunking)
        .unwrap()
        .before_completed();
    assert_eq!(
        count,
        derive_deltas(turn, &built, &chunking)
            .unwrap()
            .before_completed()
    );
    assert!(
        Reply::cut(sketch.clone(), count, ClientError::Transport)
            .chunking(chunking)
            .build()
            .is_ok()
    );
    assert_eq!(
        Reply::hang(sketch, count + 1)
            .chunking(chunking)
            .build()
            .unwrap_err(),
        MockBuildError::DeltaCountOutOfRange {
            requested: count + 1,
            available: count
        }
    );
}

#[test]
fn given_a_sketch_in_a_reply_when_built_and_when_served_then_the_source_is_touched_only_by_the_call()
 {
    let source = crate::SequentialIdSource::new();
    let client = crate::MockClient::builder(settings(), everything_caps())
        .id_source(source.clone())
        .build();
    let (sketch, _) = sketched_and_built();
    client.push_reply(Reply::success(sketch).build().unwrap());
    assert_eq!(source.given(), 0);
    run(&client, &request());
    assert_eq!(source.given(), 2);
}

#[test]
fn given_a_usage_that_breaks_counting_in_a_sketch_when_a_reply_is_built_then_usage_breaks_counting()
{
    let sketch = Sketch::new()
        .text("x")
        .usage(Usage::of_cache_read_input_tokens(1));
    assert_eq!(
        Reply::success(sketch).build().unwrap_err(),
        MockBuildError::UsageBreaksCounting
    );
}

#[test]
fn given_an_invalid_sketch_when_deltas_are_derived_then_sketch_invalid() {
    assert!(matches!(
        derive_deltas(
            crate::fixtures::turn_id(),
            Sketch::new().text(""),
            &Chunking::none()
        ),
        Err(MockBuildError::SketchInvalid { .. })
    ));
}
