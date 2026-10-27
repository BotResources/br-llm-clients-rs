use br_llm_messages::{ClosureReason, ContentBlock, RuntimeToolCall};

use super::test_support::{
    blocks_response, call, call_external, calls_response, closure, exhausted_response, fixed_turn,
    human_turn_of, invalid_reason, render, render_ok, result, results, strs, target_turn, view,
};
use super::{RenderedPart, RenderedResultPart};
use crate::thread_fixtures::{Fixture, text_block};

const FIRST: &str = "01890a5d-ac96-774b-bcce-b00000000001";
const SECOND: &str = "01890a5d-ac96-774b-bcce-b00000000002";

fn call_block(call: &RuntimeToolCall) -> ContentBlock {
    ContentBlock::RuntimeToolCall(call.clone())
}

#[test]
fn given_an_open_turn_with_an_unanswered_call_when_rendered_then_invalid_messages_names_the_call() {
    let fixture = Fixture::new();
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![calls_response(&[call(1, "alpha")])],
    )];
    let reason = invalid_reason(render(&fixture, turns, fixed_turn(1)));
    assert!(reason.contains(FIRST), "{reason}");
}

#[test]
fn given_an_open_turn_that_is_not_the_reply_turn_when_rendered_then_it_is_refused_too() {
    let fixture = Fixture::new();
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![calls_response(&[call(1, "alpha")])],
        ),
        human_turn_of("next"),
    ];
    let reason = invalid_reason(render(&fixture, turns, fixed_turn(9)));
    assert!(reason.contains(FIRST), "{reason}");
}

#[test]
fn given_an_open_turn_whose_response_ended_otherwise_when_rendered_then_it_is_refused_whatever_the_ending()
 {
    let fixture = Fixture::new();
    let finished = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![blocks_response(vec![call_block(&call(1, "alpha"))])],
    )];
    assert!(invalid_reason(render(&fixture, finished, fixed_turn(9))).contains(FIRST));
    let exhausted = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![exhausted_response(vec![call_block(&call(1, "alpha"))])],
    )];
    assert!(invalid_reason(render(&fixture, exhausted, fixed_turn(9))).contains(FIRST));
}

#[test]
fn given_an_open_turn_with_one_call_answered_when_rendered_then_the_reason_names_the_other_call() {
    let fixture = Fixture::new();
    let (first, second) = (call(1, "alpha"), call(2, "beta"));
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(&[first.clone(), second]),
            results(vec![result(&first, false, vec![text_block("ok")])]),
        ],
    )];
    let reason = invalid_reason(render(&fixture, turns, fixed_turn(1)));
    assert!(reason.contains(SECOND), "{reason}");
    assert!(!reason.contains(FIRST), "{reason}");
}

#[test]
fn given_an_open_turn_with_every_call_answered_when_rendered_then_accepted() {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            results(vec![result(&only, false, vec![text_block("ok")])]),
        ],
    )];
    assert!(render(&fixture, turns, fixed_turn(1)).is_ok());
}

#[test]
fn given_a_closed_turn_with_an_unanswered_call_when_rendered_then_a_failed_result_is_built_before_the_closure()
 {
    let fixture = Fixture::new();
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![
                calls_response(&[call(1, "alpha")]),
                closure(ClosureReason::failed("crashed").unwrap()),
            ],
        ),
        human_turn_of("next"),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history),
        vec![
            (
                'A',
                strs(&["call call_01890a5dac96774bbcceb00000000001 alpha"])
            ),
            (
                'U',
                strs(&[
                    "result call_01890a5dac96774bbcceb00000000001 alpha error=true [No result was recorded for this tool call because its turn ended before a result came.]",
                    "<notice>Your previous turn failed. Detail: crashed</notice>",
                    "next"
                ])
            ),
        ]
    );
}

#[test]
fn given_a_built_result_when_inspected_then_it_is_a_failure_with_the_exact_text_for_the_call() {
    let fixture = Fixture::new();
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(&[call_external(1, "alpha", "toolu_9")]),
            closure(ClosureReason::cancelled()),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    let parts = history.messages()[1].parts();
    match &parts[0] {
        RenderedPart::ToolResult {
            wire_id,
            tool_name,
            is_error,
            parts,
        } => {
            assert_eq!(wire_id, "toolu_9");
            assert_eq!(tool_name.as_str(), "alpha");
            assert!(*is_error);
            assert_eq!(
                parts,
                &vec![RenderedResultPart::Text(
                    "No result was recorded for this tool call because its turn ended before a result came."
                        .to_owned()
                )]
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn given_a_closed_turn_with_a_partial_delivery_when_rendered_then_the_repair_takes_its_place_by_call_order()
 {
    let fixture = Fixture::new();
    let (first, second) = (call(1, "alpha"), call(2, "beta"));
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![
                calls_response(&[first, second.clone()]),
                results(vec![result(&second, false, vec![text_block("from beta")])]),
                closure(ClosureReason::cancelled()),
            ],
        ),
        human_turn_of("after"),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history).get(1),
        Some(&(
            'U',
            strs(&[
                "result call_01890a5dac96774bbcceb00000000001 alpha error=true [No result was recorded for this tool call because its turn ended before a result came.]",
                "result call_01890a5dac96774bbcceb00000000002 beta error=false [from beta]",
                "<notice>Your previous turn was cancelled.</notice>",
                "after"
            ])
        ))
    );
}

#[test]
fn given_a_built_result_for_the_later_call_when_rendered_then_the_real_result_still_comes_first() {
    let fixture = Fixture::new();
    let (first, second) = (call(1, "alpha"), call(2, "beta"));
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(&[first.clone(), second]),
            results(vec![result(&first, false, vec![text_block("from alpha")])]),
            closure(ClosureReason::interrupted()),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history).get(1),
        Some(&(
            'U',
            strs(&[
                "result call_01890a5dac96774bbcceb00000000001 alpha error=false [from alpha]",
                "result call_01890a5dac96774bbcceb00000000002 beta error=true [No result was recorded for this tool call because its turn ended before a result came.]",
                "<notice>Your previous turn was interrupted.</notice>"
            ])
        ))
    );
}

#[test]
fn given_a_closed_turn_with_every_call_answered_when_rendered_then_no_result_is_built() {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            results(vec![result(&only, false, vec![text_block("ok")])]),
            closure(ClosureReason::cancelled()),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history).get(1),
        Some(&(
            'U',
            strs(&[
                "result call_01890a5dac96774bbcceb00000000001 alpha error=false [ok]",
                "<notice>Your previous turn was cancelled.</notice>"
            ])
        ))
    );
}

#[test]
fn given_a_closed_turn_that_is_not_the_reply_turn_when_rendered_then_it_is_repaired_too() {
    let fixture = Fixture::new();
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![
                calls_response(&[call(1, "alpha")]),
                closure(ClosureReason::budget_exhausted()),
            ],
        ),
        target_turn(
            &fixture,
            fixed_turn(2),
            vec![blocks_response(vec![ContentBlock::Text(
                br_llm_messages::TextBlock::new(crate::thread_fixtures::block_id(), "later")
                    .unwrap(),
            )])],
        ),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(2));
    assert_eq!(
        view(&history),
        vec![
            (
                'A',
                strs(&["call call_01890a5dac96774bbcceb00000000001 alpha"])
            ),
            (
                'U',
                strs(&[
                    "result call_01890a5dac96774bbcceb00000000001 alpha error=true [No result was recorded for this tool call because its turn ended before a result came.]",
                    "<notice>Your previous turn stopped because its budget was exhausted.</notice>"
                ])
            ),
            ('A', strs(&["later"])),
        ]
    );
}
