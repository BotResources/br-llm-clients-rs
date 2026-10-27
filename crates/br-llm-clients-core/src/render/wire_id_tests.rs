use super::test_support::{
    call, call_external, calls_response, fixed_turn, render_ok, result, results, strs, target_turn,
    view,
};
use crate::thread_fixtures::{Fixture, text_block};

fn answered_turn(
    fixture: &Fixture,
    turn: u32,
    calls: &[br_llm_messages::RuntimeToolCall],
) -> br_llm_messages::Turn {
    let answers = calls
        .iter()
        .map(|call| result(call, false, vec![text_block("ok")]))
        .collect();
    target_turn(
        fixture,
        fixed_turn(turn),
        vec![calls_response(calls), results(answers)],
    )
}

fn ids_of(history: &super::RenderedHistory) -> Vec<String> {
    let mut ids = Vec::new();
    for (_, parts) in view(history) {
        for part in parts {
            if part.starts_with("call ") || part.starts_with("result ") {
                let id = part.split(' ').nth(1).unwrap_or_default().to_owned();
                ids.push(id);
            }
        }
    }
    ids
}

#[test]
fn given_a_call_without_external_id_when_rendered_then_the_id_is_call_and_the_block_id_without_hyphens()
 {
    let fixture = Fixture::new();
    let turns = vec![answered_turn(&fixture, 1, &[call(1, "alpha")])];
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
                strs(&["result call_01890a5dac96774bbcceb00000000001 alpha error=false [ok]"])
            ),
        ]
    );
}

#[test]
fn given_a_call_with_an_external_id_when_rendered_then_call_and_result_carry_it() {
    let fixture = Fixture::new();
    let turns = vec![answered_turn(
        &fixture,
        1,
        &[call_external(1, "alpha", "toolu_01A")],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(ids_of(&history), strs(&["toolu_01A", "toolu_01A"]));
}

#[test]
fn given_three_calls_with_one_external_id_when_rendered_then_only_the_first_keeps_it() {
    let fixture = Fixture::new();
    let calls = [
        call_external(1, "alpha", "dup"),
        call_external(2, "beta", "dup"),
        call_external(3, "gamma", "dup"),
    ];
    let turns = vec![answered_turn(&fixture, 1, &calls)];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        ids_of(&history),
        strs(&[
            "dup",
            "call_01890a5dac96774bbcceb00000000002",
            "call_01890a5dac96774bbcceb00000000003",
            "dup",
            "call_01890a5dac96774bbcceb00000000002",
            "call_01890a5dac96774bbcceb00000000003",
        ])
    );
}

#[test]
fn given_the_same_external_id_in_two_turns_when_rendered_then_the_later_call_gets_a_generated_id() {
    let fixture = Fixture::new();
    let turns = vec![
        answered_turn(&fixture, 1, &[call_external(1, "alpha", "shared")]),
        answered_turn(&fixture, 2, &[call_external(2, "alpha", "shared")]),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        ids_of(&history),
        strs(&[
            "shared",
            "shared",
            "call_01890a5dac96774bbcceb00000000002",
            "call_01890a5dac96774bbcceb00000000002",
        ])
    );
}

#[test]
fn given_a_generated_id_equal_to_the_external_id_of_a_later_call_when_rendered_then_the_generated_one_is_suffixed()
 {
    let fixture = Fixture::new();
    let calls = [
        call(1, "alpha"),
        call_external(2, "beta", "call_01890a5dac96774bbcceb00000000001"),
    ];
    let turns = vec![answered_turn(&fixture, 1, &calls)];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        ids_of(&history),
        strs(&[
            "call_01890a5dac96774bbcceb00000000001_1",
            "call_01890a5dac96774bbcceb00000000001",
            "call_01890a5dac96774bbcceb00000000001_1",
            "call_01890a5dac96774bbcceb00000000001",
        ])
    );
}

#[test]
fn given_a_generated_id_equal_to_an_external_id_in_a_later_turn_when_rendered_then_the_generated_one_is_suffixed()
 {
    let fixture = Fixture::new();
    let turns = vec![
        answered_turn(&fixture, 1, &[call(1, "alpha")]),
        answered_turn(
            &fixture,
            2,
            &[call_external(
                2,
                "alpha",
                "call_01890a5dac96774bbcceb00000000001",
            )],
        ),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        ids_of(&history),
        strs(&[
            "call_01890a5dac96774bbcceb00000000001_1",
            "call_01890a5dac96774bbcceb00000000001_1",
            "call_01890a5dac96774bbcceb00000000001",
            "call_01890a5dac96774bbcceb00000000001",
        ])
    );
}

#[test]
fn given_suffixed_candidates_taken_by_external_ids_when_rendered_then_the_suffix_counts_up_without_chaining()
 {
    let fixture = Fixture::new();
    let calls = [
        call(1, "alpha"),
        call_external(2, "beta", "call_01890a5dac96774bbcceb00000000001"),
        call_external(3, "gamma", "call_01890a5dac96774bbcceb00000000001_1"),
    ];
    let turns = vec![answered_turn(&fixture, 1, &calls)];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        ids_of(&history).get(..3),
        Some(
            &strs(&[
                "call_01890a5dac96774bbcceb00000000001_2",
                "call_01890a5dac96774bbcceb00000000001",
                "call_01890a5dac96774bbcceb00000000001_1",
            ])[..]
        )
    );
}

#[test]
fn given_a_generated_id_equal_to_the_external_id_of_an_earlier_call_when_rendered_then_it_is_suffixed()
 {
    let fixture = Fixture::new();
    let calls = [
        call_external(1, "alpha", "call_01890a5dac96774bbcceb00000000002"),
        call(2, "beta"),
    ];
    let turns = vec![answered_turn(&fixture, 1, &calls)];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        ids_of(&history),
        strs(&[
            "call_01890a5dac96774bbcceb00000000002",
            "call_01890a5dac96774bbcceb00000000002_1",
            "call_01890a5dac96774bbcceb00000000002",
            "call_01890a5dac96774bbcceb00000000002_1",
        ])
    );
}
