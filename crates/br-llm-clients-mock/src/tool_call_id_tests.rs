use std::collections::HashSet;

use crate::test_support::*;
use crate::tool_call_id::{rename_tool_calls, tool_call_id};

#[test]
fn given_a_prefix_and_a_sequence_when_made_then_the_id_is_prefix_dash_sequence() {
    assert_eq!(tool_call_id("call", 7, None).as_str(), "call-7");
}

#[test]
fn given_an_index_when_made_then_the_id_has_a_third_part() {
    assert_eq!(tool_call_id("call", 7, Some(2)).as_str(), "call-7-2");
}

#[test]
fn given_distinct_sequence_numbers_when_made_then_the_ids_are_distinct() {
    let ids: HashSet<String> = (0..100)
        .map(|sequence| tool_call_id("c", sequence, None).to_string())
        .collect();
    assert_eq!(ids.len(), 100);
}

#[test]
fn given_an_empty_prefix_when_made_then_the_id_is_still_valid() {
    assert_eq!(tool_call_id("", 0, None).as_str(), "-0");
}

#[test]
fn given_a_step_with_several_tool_calls_when_renamed_then_ids_follow_the_block_order_from_zero() {
    let renamed = rename_tool_calls(&calls_step(&["x", "y", "z"]), "id", 4).unwrap();
    let ids: Vec<&str> = renamed.tool_calls().map(|call| call.id.as_str()).collect();
    assert_eq!(ids, ["id-4-0", "id-4-1", "id-4-2"]);
}

#[test]
fn given_a_renamed_step_when_compared_then_only_the_ids_changed() {
    let original = calls_step(&["x", "y"]);
    let renamed = rename_tool_calls(&original, "id", 0).unwrap();
    assert_eq!(renamed.stop_reason(), original.stop_reason());
    let names: Vec<_> = renamed.tool_calls().map(|call| call.name.clone()).collect();
    let before: Vec<_> = original
        .tool_calls()
        .map(|call| call.name.clone())
        .collect();
    assert_eq!(names, before);
    let arguments: Vec<_> = renamed
        .tool_calls()
        .map(|call| call.arguments.clone())
        .collect();
    let args_before: Vec<_> = original
        .tool_calls()
        .map(|call| call.arguments.clone())
        .collect();
    assert_eq!(arguments, args_before);
}

#[test]
fn given_a_step_without_tool_call_when_renamed_then_it_is_unchanged() {
    let original = text_step("hello");
    assert_eq!(rename_tool_calls(&original, "id", 3).unwrap(), original);
}
