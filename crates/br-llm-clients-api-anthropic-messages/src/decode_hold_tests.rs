use br_llm_clients_common_call::ProviderEvent;
use br_llm_messages::StopReason;

use crate::test_decode::*;

fn open_tool(session: &mut Session, index: u64, id: &str) {
    session.ok(&format!(
        r#"{{"type":"content_block_start","index":{index},"content_block":{{"type":"tool_use","id":"{id}","name":"n","input":{{}}}}}}"#
    ));
}

fn stop_block(session: &mut Session, index: u64) -> Vec<ProviderEvent> {
    session.ok(&format!(
        r#"{{"type":"content_block_stop","index":{index}}}"#
    ))
}

fn args(session: &mut Session, index: u64, json: &str) {
    session.ok(&format!(
        r#"{{"type":"content_block_delta","index":{index},"delta":{{"type":"input_json_delta","partial_json":"{json}"}}}}"#
    ));
}

fn finish(session: &mut Session, reason: Option<&str>) -> Vec<ProviderEvent> {
    if let Some(reason) = reason {
        session.ok(&format!(
            r#"{{"type":"message_delta","delta":{{"stop_reason":"{reason}"}}}}"#
        ));
    }
    session.ok(r#"{"type":"message_stop"}"#)
}

#[test]
fn given_a_tool_call_with_no_argument_text_when_it_stops_then_nothing_is_given() {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    assert!(stop_block(&mut session, 0).is_empty());
}

#[test]
fn given_a_held_tool_call_when_the_stop_reason_is_tool_use_then_message_stop_gives_the_empty_object_and_the_end_before_stop()
 {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    stop_block(&mut session, 0);
    assert_eq!(
        finish(&mut session, Some("tool_use")),
        vec![
            args_delta("0", "{}"),
            end_of("0"),
            stop(StopReason::AwaitingToolResults),
            ProviderEvent::Complete,
        ]
    );
}

#[test]
fn given_only_empty_argument_deltas_when_the_call_stops_then_it_is_held_like_a_call_with_none() {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    args(&mut session, 0, "");
    args(&mut session, 0, "");
    assert!(stop_block(&mut session, 0).is_empty());
    assert_eq!(finish(&mut session, Some("tool_use")).len(), 4);
}

#[test]
fn given_several_held_calls_when_message_stop_arrives_then_they_come_in_the_order_of_their_stops() {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    open_tool(&mut session, 1, "t1");
    open_tool(&mut session, 2, "t2");
    stop_block(&mut session, 2);
    stop_block(&mut session, 0);
    stop_block(&mut session, 1);
    assert_eq!(
        finish(&mut session, Some("tool_use")),
        vec![
            args_delta("2", "{}"),
            end_of("2"),
            args_delta("0", "{}"),
            end_of("0"),
            args_delta("1", "{}"),
            end_of("1"),
            stop(StopReason::AwaitingToolResults),
            ProviderEvent::Complete,
        ]
    );
}

#[test]
fn given_a_call_with_arguments_beside_a_held_one_when_decoded_then_only_the_held_one_waits() {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    open_tool(&mut session, 1, "t1");
    args(&mut session, 1, "{\\\"a\\\":1}");
    assert_eq!(stop_block(&mut session, 1), vec![end_of("1")]);
    assert!(stop_block(&mut session, 0).is_empty());
    assert_eq!(
        finish(&mut session, Some("tool_use")),
        vec![
            args_delta("0", "{}"),
            end_of("0"),
            stop(StopReason::AwaitingToolResults),
            ProviderEvent::Complete,
        ]
    );
}

#[test]
fn given_a_held_call_when_the_stop_reason_is_not_tool_use_then_neither_the_object_nor_the_end_is_given()
 {
    let reasons = [
        ("max_tokens", StopReason::MaxTokens),
        ("end_turn", StopReason::EndTurn),
        ("refusal", StopReason::Refusal),
        ("stop_sequence", StopReason::StopSequence),
        (
            "pause_turn",
            StopReason::Other {
                reason: "pause_turn".to_owned(),
            },
        ),
        (
            "model_context_window_exceeded",
            StopReason::Other {
                reason: "model_context_window_exceeded".to_owned(),
            },
        ),
        (
            "brand_new",
            StopReason::Other {
                reason: "brand_new".to_owned(),
            },
        ),
    ];
    for (reason, mapped) in reasons {
        let mut session = Session::started();
        open_tool(&mut session, 0, "t0");
        stop_block(&mut session, 0);
        assert_eq!(
            finish(&mut session, Some(reason)),
            vec![stop(mapped), ProviderEvent::Complete],
            "{reason}"
        );
    }
}

#[test]
fn given_a_held_call_and_no_stop_reason_when_message_stop_arrives_then_only_complete_is_given() {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    stop_block(&mut session, 0);
    assert_eq!(finish(&mut session, None), vec![ProviderEvent::Complete]);
}

#[test]
fn given_a_later_stop_reason_when_tool_use_was_recorded_first_then_the_last_one_decides_the_hold() {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    stop_block(&mut session, 0);
    session.ok(r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"}}"#);
    session.ok(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#);
    assert_eq!(
        session.ok(r#"{"type":"message_stop"}"#),
        vec![stop(StopReason::EndTurn), ProviderEvent::Complete]
    );
}

#[test]
fn given_a_held_call_when_a_delta_a_second_stop_or_a_start_hits_its_index_then_it_is_a_payload_error()
 {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    stop_block(&mut session, 0);
    session.rejected(
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{}"}}"#,
    );
    session.rejected(r#"{"type":"content_block_stop","index":0}"#);
    session.rejected(
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"x","name":"n","input":{}}}"#,
    );
}

#[test]
fn given_blocks_left_open_when_message_stop_arrives_then_no_end_is_given_for_them() {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    args(&mut session, 0, "{\\\"a\\\"");
    session.ok(r#"{"type":"content_block_start","index":1,"content_block":{"type":"text"}}"#);
    assert_eq!(
        finish(&mut session, Some("max_tokens")),
        vec![stop(StopReason::MaxTokens), ProviderEvent::Complete]
    );
}

#[test]
fn given_a_held_call_when_the_stream_never_stops_then_end_reports_the_marker_missing() {
    let mut session = Session::started();
    open_tool(&mut session, 0, "t0");
    stop_block(&mut session, 0);
    assert_eq!(
        session.end(),
        Ok(br_llm_clients_common_call::End::MarkerMissing)
    );
}
