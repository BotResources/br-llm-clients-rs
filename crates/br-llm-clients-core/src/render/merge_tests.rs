use br_llm_messages::{ClosureReason, ContentBlock, ThinkingBlock};

use super::test_support::{
    blocks_response, call, calls_response, closure, exhausted_response, fixed_turn, human_turn_of,
    invalid_reason, render, render_ok, result, results, strs, target_turn, text, view,
};
use crate::thread_fixtures::{Fixture, block_id, text_block};

fn thinking_only() -> Vec<ContentBlock> {
    vec![ContentBlock::Thinking(
        ThinkingBlock::new(block_id(), "pondering").unwrap(),
    )]
}

#[test]
fn given_human_then_model_then_human_when_rendered_then_roles_alternate() {
    let fixture = Fixture::new();
    let turns = vec![
        human_turn_of("one"),
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![blocks_response(vec![text("two")])],
        ),
        human_turn_of("three"),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            ('U', strs(&["one"])),
            ('A', strs(&["two"])),
            ('U', strs(&["three"]))
        ]
    );
}

#[test]
fn given_a_history_starting_and_ending_with_the_model_when_rendered_then_it_is_not_judged() {
    let fixture = Fixture::new();
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![blocks_response(vec![text("only")])],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(view(&history), vec![('A', strs(&["only"]))]);
}

#[test]
fn given_consecutive_user_entries_across_turns_when_rendered_then_one_user_message() {
    let fixture = Fixture::new();
    let turns = vec![human_turn_of("a"), human_turn_of("b"), human_turn_of("c")];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(view(&history), vec![('U', strs(&["a", "b", "c"]))]);
}

#[test]
fn given_consecutive_model_responses_in_one_turn_when_rendered_then_one_assistant_message() {
    let fixture = Fixture::new();
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            exhausted_response(vec![text("first")]),
            blocks_response(vec![text("second")]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(view(&history), vec![('A', strs(&["first", "second"]))]);
}

#[test]
fn given_model_responses_in_two_turns_when_rendered_then_one_assistant_message() {
    let fixture = Fixture::new();
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![
                blocks_response(vec![text("first")]),
                closure(ClosureReason::completed()),
            ],
        ),
        target_turn(
            &fixture,
            fixed_turn(2),
            vec![blocks_response(vec![text("second")])],
        ),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(view(&history), vec![('A', strs(&["first", "second"]))]);
}

#[test]
fn given_an_entry_that_renders_nothing_between_two_user_entries_when_rendered_then_they_merge() {
    let fixture = Fixture::new();
    let turns = vec![
        human_turn_of("before"),
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![
                blocks_response(thinking_only()),
                closure(ClosureReason::completed().with_detail("done").unwrap()),
            ],
        ),
        human_turn_of("after"),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(view(&history), vec![('U', strs(&["before", "after"]))]);
}

#[test]
fn given_a_model_response_with_nothing_to_send_between_two_model_responses_when_rendered_then_they_merge()
 {
    let fixture = Fixture::new();
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![blocks_response(vec![text("a")])],
        ),
        target_turn(
            &fixture,
            fixed_turn(2),
            vec![blocks_response(thinking_only())],
        ),
        target_turn(
            &fixture,
            fixed_turn(3),
            vec![blocks_response(vec![text("b")])],
        ),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(view(&history), vec![('A', strs(&["a", "b"]))]);
}

#[test]
fn given_a_history_that_renders_nothing_when_rendered_then_invalid_messages() {
    let fixture = Fixture::new();
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![closure(ClosureReason::completed())],
    )];
    let reason = invalid_reason(render(&fixture, turns, fixed_turn(9)));
    assert_eq!(reason, "the history renders no message");
}

#[test]
fn given_a_history_of_thinking_only_in_another_turn_when_rendered_then_invalid_messages() {
    let fixture = Fixture::new();
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![blocks_response(thinking_only())],
    )];
    invalid_reason(render(&fixture, turns, fixed_turn(9)));
}

#[test]
fn given_results_listed_against_the_order_of_their_calls_when_rendered_then_they_follow_the_order_of_the_calls()
 {
    let fixture = Fixture::new();
    let first = call(1, "alpha");
    let second = call(2, "beta");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(&[first.clone(), second.clone()]),
            results(vec![
                result(&second, false, vec![text_block("from beta")]),
                result(&first, false, vec![text_block("from alpha")]),
            ]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            (
                'A',
                strs(&[
                    "call call_01890a5dac96774bbcceb00000000001 alpha",
                    "call call_01890a5dac96774bbcceb00000000002 beta"
                ])
            ),
            (
                'U',
                strs(&[
                    "result call_01890a5dac96774bbcceb00000000001 alpha error=false [from alpha]",
                    "result call_01890a5dac96774bbcceb00000000002 beta error=false [from beta]"
                ])
            ),
        ]
    );
}

#[test]
fn given_results_followed_by_other_entries_of_the_user_when_rendered_then_results_come_first() {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![
                calls_response(std::slice::from_ref(&only)),
                results(vec![result(&only, false, vec![text_block("out")])]),
            ],
        ),
        human_turn_of("next"),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history).get(1),
        Some(&(
            'U',
            strs(&[
                "result call_01890a5dac96774bbcceb00000000001 alpha error=false [out]",
                "next"
            ])
        ))
    );
}

#[test]
fn given_the_same_request_when_rendered_twice_then_the_renderings_are_equal() {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![
        human_turn_of("go"),
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![
                calls_response(std::slice::from_ref(&only)),
                results(vec![result(&only, true, vec![text_block("bad")])]),
            ],
        ),
    ];
    let request = super::test_support::request_for(&fixture, turns, fixed_turn(1));
    assert_eq!(
        super::render_history(&request).unwrap(),
        super::render_history(&request).unwrap()
    );
}
