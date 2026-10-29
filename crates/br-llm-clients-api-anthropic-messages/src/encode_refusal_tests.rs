use br_llm_clients_core::{ClientError, OutputMode, ParameterKind};
use br_llm_messages::{
    ContentBlock, OpaqueJson, ProviderToolCall, ProviderToolExecutionRecord, ReplayFormat,
    ReplayMaterial, ReplayPayload, Turn,
};
use serde_json::json;

use crate::test_fixtures::*;
use crate::test_thread::*;

fn reason_of(error: ClientError) -> String {
    match error {
        ClientError::InvalidMessages { reason } => {
            assert!(!reason.is_empty());
            reason
        }
        other => panic!("expected InvalidMessages, got {other:?}"),
    }
}

fn refused(turns: Vec<Turn>) -> String {
    reason_of(refusal_of(&dialect(), &request(turns)))
}

fn json_material(format: &str) -> ReplayMaterial {
    let value = OpaqueJson::parse(r#"{"sig":"x"}"#).expect("json");
    ReplayMaterial::new(
        ReplayFormat::new(format).expect("a format"),
        ReplayPayload::json(value).expect("a json payload"),
    )
}

#[test]
fn given_a_last_assistant_message_when_encoded_then_it_is_refused_as_a_prefill() {
    refused(vec![user("a"), assistant_text("b")]);
}

#[test]
fn given_a_last_assistant_message_after_a_tool_loop_when_encoded_then_it_is_refused() {
    let call = call("t1", "lookup", json!({}));
    refused(vec![
        user("go"),
        earlier(vec![
            calling(vec![call_block(&call)]),
            results(vec![result_of(&call, texts(&["r"]), false)]),
            answer(vec![text("done")]),
        ]),
    ]);
}

#[test]
fn given_a_thinking_without_replay_material_in_the_turn_in_progress_when_encoded_then_it_is_refused()
 {
    refused(in_progress_with(thinking("hm", None)));
}

#[test]
fn given_a_thinking_whose_replay_material_has_another_format_when_encoded_then_it_is_refused() {
    refused(in_progress_with(thinking(
        "hm",
        Some(material(
            "br-llm-clients-api-openai-responses/encrypted",
            "sig",
        )),
    )));
    refused(in_progress_with(thinking("hm", Some(redacted_data("sig")))));
}

#[test]
fn given_a_redacted_thinking_whose_replay_material_has_another_format_when_encoded_then_it_is_refused()
 {
    refused(in_progress_with(redacted(material("someone/else", "data"))));
    refused(in_progress_with(redacted(signature("data"))));
}

#[test]
fn given_replay_material_of_the_right_format_that_is_not_text_when_encoded_then_it_is_refused() {
    refused(in_progress_with(thinking(
        "hm",
        Some(json_material(crate::replay_formats::SIGNATURE_FORMAT)),
    )));
    refused(in_progress_with(redacted(json_material(
        crate::replay_formats::REDACTED_DATA_FORMAT,
    ))));
}

#[test]
fn given_a_provider_tool_block_in_the_turn_in_progress_when_encoded_then_it_is_refused_naming_its_kind()
 {
    let record = ProviderToolExecutionRecord::new(block_id())
        .with_content(texts(&["record-secret"]))
        .expect("a provider record");
    for (block, kind) in [
        (
            ContentBlock::ProviderToolCall(ProviderToolCall::new(block_id())),
            "provider_tool_call",
        ),
        (
            ContentBlock::ProviderToolExecutionRecord(record),
            "provider_tool_execution_record",
        ),
    ] {
        let reason = refused(in_progress_with(block));
        assert!(reason.contains(kind), "{reason}");
        assert!(!reason.contains("secret"), "{reason}");
    }
}

#[test]
fn given_a_refusal_when_encoded_then_the_reason_holds_no_content_of_the_prompt() {
    let reason = refused(vec![
        user("a very private question"),
        assistant_text("a very private answer"),
    ]);
    assert!(!reason.contains("private"));
    let reason = refused(in_progress_with(thinking(
        "private thought",
        Some(material("x/y", "private-signature")),
    )));
    assert!(!reason.contains("private"));
}

#[test]
fn given_a_structured_output_and_a_declared_tool_when_encoded_then_it_is_refused() {
    let request = request_with(
        None,
        vec![user("a")],
        vec![tool("lookup", "", vec![])],
        structured(None, vec![param("v", "", true, ParameterKind::String)]),
    );
    assert_eq!(
        refusal_of(&dialect(), &request),
        ClientError::StructuredOutputWithToolsNotSupported
    );
}

#[test]
fn given_a_structured_output_and_tool_calls_in_history_but_no_tool_when_encoded_then_it_is_accepted()
 {
    let call = call("t1", "lookup", json!({}));
    let request = request_with(
        None,
        vec![
            user("go"),
            in_progress(vec![
                calling(vec![call_block(&call)]),
                results(vec![result_of(&call, texts(&["r"]), false)]),
            ]),
        ],
        vec![],
        structured(None, vec![]),
    );
    assert!(encode(&dialect(), &request).is_ok());
}

#[test]
fn given_a_plain_output_with_tools_when_encoded_then_it_is_accepted() {
    let request = request_with(
        None,
        vec![user("a")],
        vec![tool("lookup", "", vec![])],
        OutputMode::Plain,
    );
    assert!(encode(&dialect(), &request).is_ok());
}
