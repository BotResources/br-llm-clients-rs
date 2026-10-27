use br_llm_messages::{ClosureReason, TurnEntry};

use super::test_support::{
    blocks_response, call, calls_response, closure, fixed_turn, human_entry, human_turn_of,
    render_ok, result, results, strs, target_turn, text, view,
};
use crate::thread_fixtures::{Fixture, as_turn_entry, input_turn, text_block};

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
fn given_a_result_that_arrives_after_the_closure_when_rendered_then_the_call_is_answered_and_nothing_is_built()
 {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![
                calls_response(std::slice::from_ref(&only)),
                closure(ClosureReason::cancelled()),
                results(vec![result(&only, false, vec![text_block("late")])]),
            ],
        ),
        human_turn_of("after"),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&[&format!("call {FIRST} alpha")])),
            (
                'U',
                strs(&[
                    &format!("result {FIRST} alpha error=false [late]"),
                    "<notice>Your previous turn was cancelled.</notice>",
                    "after"
                ])
            ),
        ]
    );
}

#[test]
fn given_a_late_result_for_one_of_two_calls_when_rendered_then_only_the_other_call_is_repaired_in_call_order()
 {
    let fixture = Fixture::new();
    let (first, second) = (call(1, "alpha"), call(2, "beta"));
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(&[first, second.clone()]),
            closure(ClosureReason::interrupted()),
            results(vec![result(&second, false, vec![text_block("late beta")])]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history).get(1),
        Some(&(
            'U',
            strs(&[
                &format!("result {FIRST} alpha error=true [{REPAIR}]"),
                &format!("result {SECOND} beta error=false [late beta]"),
                "<notice>Your previous turn was interrupted.</notice>"
            ])
        ))
    );
}

#[test]
fn given_a_late_result_for_the_first_call_when_rendered_then_the_built_result_for_the_second_follows_it()
 {
    let fixture = Fixture::new();
    let (first, second) = (call(1, "alpha"), call(2, "beta"));
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(&[first.clone(), second]),
            closure(ClosureReason::failed("crashed").unwrap()),
            results(vec![result(&first, true, vec![text_block("late alpha")])]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history).get(1),
        Some(&(
            'U',
            strs(&[
                &format!("result {FIRST} alpha error=true [late alpha]"),
                &format!("result {SECOND} beta error=true [{REPAIR}]"),
                "<notice>Your previous turn failed. Detail: crashed</notice>"
            ])
        ))
    );
}

#[test]
fn given_entries_in_a_free_order_when_rendered_then_each_entry_is_rendered_in_turn_order() {
    let fixture = Fixture::new();
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            incoming("first question"),
            reply("first answer"),
            incoming("second question"),
            incoming("third question"),
            reply("second answer"),
            reply("third answer"),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('U', strs(&["first question"])),
            ('A', strs(&["first answer"])),
            ('U', strs(&["second question", "third question"])),
            ('A', strs(&["second answer", "third answer"])),
        ]
    );
}

#[test]
fn given_an_incoming_entry_between_a_call_and_its_result_when_rendered_then_the_result_comes_first()
{
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            incoming("meanwhile"),
            results(vec![result(&only, false, vec![text_block("done")])]),
            reply("final"),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&[&format!("call {FIRST} alpha")])),
            (
                'U',
                strs(&[
                    &format!("result {FIRST} alpha error=false [done]"),
                    "meanwhile"
                ])
            ),
            ('A', strs(&["final"])),
        ]
    );
}

#[test]
fn given_an_incoming_entry_after_the_result_when_rendered_then_it_follows_the_result() {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            results(vec![result(&only, false, vec![text_block("done")])]),
            incoming("afterwards"),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history).get(1),
        Some(&(
            'U',
            strs(&[
                &format!("result {FIRST} alpha error=false [done]"),
                "afterwards"
            ])
        ))
    );
}

#[test]
fn given_an_incoming_entry_after_a_closure_when_rendered_then_it_follows_the_closure_notice() {
    let fixture = Fixture::new();
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![reply("hi"), closure(ClosureReason::cancelled())],
        ),
        input_turn(vec![human_entry("one"), human_entry("two")]),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&["hi"])),
            (
                'U',
                strs(&[
                    "<notice>Your previous turn was cancelled.</notice>",
                    "one",
                    "two"
                ])
            ),
        ]
    );
}

#[test]
fn given_a_tool_name_of_256_characters_when_a_call_is_rendered_then_the_name_is_untouched() {
    let fixture = Fixture::new();
    let name = "a".repeat(256);
    let only = call(1, &name);
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            results(vec![result(&only, false, vec![text_block("ok")])]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('A', strs(&[&format!("call {FIRST} {}", "a".repeat(256))])),
            (
                'U',
                strs(&[&format!(
                    "result {FIRST} {} error=false [ok]",
                    "a".repeat(256)
                )])
            ),
        ]
    );
    assert!(matches!(
        &history.messages()[0].parts()[0],
        super::RenderedPart::ToolCall { call, .. } if call.name().as_str().len() == 256
    ));
}
