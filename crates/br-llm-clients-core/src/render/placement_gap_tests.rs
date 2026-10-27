use br_llm_messages::{ClosureReason, ContentBlock, ThinkingBlock};

use super::test_support::{
    blocks_response, call, calls_response, closure, fixed_turn, render_ok, result, results, strs,
    target_turn, text, view,
};
use crate::thread_fixtures::{Fixture, block_id, text_block};

const FIRST: &str = "call_01890a5dac96774bbcceb00000000001";
const SECOND: &str = "call_01890a5dac96774bbcceb00000000002";
const REPAIR: &str =
    "No result was recorded for this tool call because its turn ended before a result came.";

fn thinking_only() -> Vec<ContentBlock> {
    vec![ContentBlock::Thinking(
        ThinkingBlock::new(block_id(), "pondering").unwrap(),
    )]
}

#[test]
fn given_an_entry_rendering_nothing_between_a_call_and_its_result_when_rendered_then_the_responses_merge_and_the_result_follows_the_merged_message()
 {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            blocks_response(thinking_only()),
            blocks_response(vec![text("resp2")]),
            results(vec![result(&only, false, vec![text_block("R1")])]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&[&format!("call {FIRST} alpha"), "resp2"])),
            (
                'U',
                strs(&[&format!("result {FIRST} alpha error=false [R1]")])
            ),
        ]
    );
}

#[test]
fn given_two_calls_with_one_delivered_result_and_one_unresolved_at_closure_when_rendered_then_the_result_and_the_repair_share_a_message_in_call_order_before_the_notice()
 {
    let fixture = Fixture::new();
    let (first, second) = (call(1, "alpha"), call(2, "beta"));
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(&[first.clone(), second.clone()]),
            results(vec![result(&second, false, vec![text_block("R2")])]),
            closure(ClosureReason::interrupted()),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            (
                'A',
                strs(&[
                    &format!("call {FIRST} alpha"),
                    &format!("call {SECOND} beta")
                ])
            ),
            (
                'U',
                strs(&[
                    &format!("result {FIRST} alpha error=true [{REPAIR}]"),
                    &format!("result {SECOND} beta error=false [R2]"),
                    "<notice>Your previous turn was interrupted.</notice>"
                ])
            ),
        ]
    );
}
