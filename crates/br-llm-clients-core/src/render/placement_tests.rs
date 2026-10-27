use br_llm_messages::{ClosureReason, TurnEntry};

use super::test_support::{
    blocks_response, call, calls_response, closure, fixed_turn, human_entry, render_ok, result,
    results, strs, target_turn, text, view,
};
use crate::thread_fixtures::{Fixture, as_turn_entry, text_block};

const FIRST: &str = "call_01890a5dac96774bbcceb00000000001";
const SECOND: &str = "call_01890a5dac96774bbcceb00000000002";
const REPAIR: &str =
    "No result was recorded for this tool call because its turn ended before a result came.";

fn incoming(value: &str) -> TurnEntry {
    as_turn_entry(human_entry(value))
}

fn reply(value: &str) -> TurnEntry {
    blocks_response(vec![text(value)])
}

#[test]
fn given_a_result_recorded_after_a_later_response_when_rendered_then_it_goes_in_the_user_message_right_after_its_call()
 {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            incoming("X"),
            reply("resp2"),
            results(vec![result(&only, false, vec![text_block("R1")])]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&[&format!("call {FIRST} alpha")])),
            (
                'U',
                strs(&[&format!("result {FIRST} alpha error=false [R1]"), "X"])
            ),
            ('A', strs(&["resp2"])),
        ]
    );
}

#[test]
fn given_results_recorded_in_reverse_order_when_rendered_then_each_goes_right_after_its_own_call() {
    let fixture = Fixture::new();
    let (first, second) = (call(1, "alpha"), call(2, "beta"));
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&first)),
            incoming("X"),
            calls_response(std::slice::from_ref(&second)),
            incoming("Y"),
            results(vec![result(&second, false, vec![text_block("R2")])]),
            results(vec![result(&first, false, vec![text_block("R1")])]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&[&format!("call {FIRST} alpha")])),
            (
                'U',
                strs(&[&format!("result {FIRST} alpha error=false [R1]"), "X"])
            ),
            ('A', strs(&[&format!("call {SECOND} beta")])),
            (
                'U',
                strs(&[&format!("result {SECOND} beta error=false [R2]"), "Y"])
            ),
        ]
    );
}

#[test]
fn given_a_result_recorded_after_the_closure_notice_when_rendered_then_it_comes_first_in_the_notice_message_and_nothing_is_built()
 {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            reply("resp2"),
            closure(ClosureReason::cancelled()),
            results(vec![result(&only, false, vec![text_block("late")])]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&[&format!("call {FIRST} alpha"), "resp2"])),
            (
                'U',
                strs(&[
                    &format!("result {FIRST} alpha error=false [late]"),
                    "<notice>Your previous turn was cancelled.</notice>"
                ])
            ),
        ]
    );
}

#[test]
fn given_an_unresolved_call_followed_by_another_response_when_rendered_then_the_repair_follows_its_call_and_the_notice_follows_the_last_response()
 {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            incoming("X"),
            reply("resp2"),
            closure(ClosureReason::interrupted()),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&[&format!("call {FIRST} alpha")])),
            (
                'U',
                strs(&[&format!("result {FIRST} alpha error=true [{REPAIR}]"), "X"])
            ),
            ('A', strs(&["resp2"])),
            (
                'U',
                strs(&["<notice>Your previous turn was interrupted.</notice>"])
            ),
        ]
    );
}

#[test]
fn given_a_result_right_after_its_call_and_a_later_incoming_entry_when_rendered_then_a_new_user_message_holds_the_result_first()
 {
    let fixture = Fixture::new();
    let (first, second) = (call(1, "alpha"), call(2, "beta"));
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&first)),
            results(vec![result(&first, false, vec![text_block("R1")])]),
            calls_response(std::slice::from_ref(&second)),
            results(vec![result(&second, false, vec![text_block("R2")])]),
            incoming("Y"),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&[&format!("call {FIRST} alpha")])),
            (
                'U',
                strs(&[&format!("result {FIRST} alpha error=false [R1]")])
            ),
            ('A', strs(&[&format!("call {SECOND} beta")])),
            (
                'U',
                strs(&[&format!("result {SECOND} beta error=false [R2]"), "Y"])
            ),
        ]
    );
}
