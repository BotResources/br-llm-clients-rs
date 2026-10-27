use br_llm_messages::{
    ClosureReason, IncomingEntry, QueuePosition, RequestAccepted, TaskCompleted, TaskFailed,
    TaskStarted, TurnEntry,
};

use super::test_support::{
    blocks_response, closure, fixed_conversation, fixed_entry, fixed_request, fixed_task,
    fixed_turn, human_turn_of, render_ok, strs, target_turn, text, view,
};
use crate::thread_fixtures::{Fixture, input_turn};

fn incoming_view(entry: IncomingEntry) -> Vec<String> {
    let fixture = Fixture::new();
    let history = render_ok(&fixture, vec![input_turn(vec![entry])], fixed_turn(9));
    assert_eq!(history.messages().len(), 1);
    view(&history).remove(0).1
}

fn in_target_turn(entry: TurnEntry) -> Vec<String> {
    let fixture = Fixture::new();
    let turns = vec![target_turn(&fixture, fixed_turn(1), vec![entry])];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(history.messages().len(), 1);
    view(&history).remove(0).1
}

fn accepted() -> RequestAccepted {
    RequestAccepted::new(fixed_entry(1), fixed_request(1))
}

fn queued() -> QueuePosition {
    QueuePosition::new(fixed_entry(2), fixed_request(1), 3).unwrap()
}

fn started() -> TaskStarted {
    TaskStarted::new(fixed_entry(3), fixed_request(1), fixed_task(2))
}

fn completed() -> TaskCompleted {
    TaskCompleted::new(fixed_entry(4), fixed_task(2))
}

fn failed(reason: &str) -> TaskFailed {
    TaskFailed::new(fixed_entry(5), fixed_task(2), reason).unwrap()
}

#[test]
fn given_a_request_accepted_when_rendered_then_the_fixed_sentence_is_one_part() {
    let literal =
        strs(&["<notice>Request 01890a5d-ac96-774b-bcce-d00000000001 was accepted.</notice>"]);
    assert_eq!(incoming_view(accepted().into()), literal);
    assert_eq!(in_target_turn(accepted().into()), literal);
}

#[test]
fn given_a_queue_position_when_rendered_then_the_number_is_written() {
    let literal = strs(&[
        "<notice>Request 01890a5d-ac96-774b-bcce-d00000000001 is number 3 in the queue.</notice>",
    ]);
    assert_eq!(incoming_view(queued().into()), literal);
    assert_eq!(in_target_turn(queued().into()), literal);
}

#[test]
fn given_a_task_started_without_conversation_when_rendered_then_no_conversation_sentence() {
    let literal = strs(&[
        "<notice>Task 01890a5d-ac96-774b-bcce-f00000000002 started for request 01890a5d-ac96-774b-bcce-d00000000001.</notice>",
    ]);
    assert_eq!(incoming_view(started().into()), literal);
    assert_eq!(in_target_turn(started().into()), literal);
}

#[test]
fn given_a_task_started_with_conversation_when_rendered_then_the_conversation_is_added() {
    let entry = started().with_conversation(fixed_conversation(3));
    let literal = strs(&[
        "<notice>Task 01890a5d-ac96-774b-bcce-f00000000002 started for request 01890a5d-ac96-774b-bcce-d00000000001. It runs in conversation 01890a5d-ac96-774b-bcce-900000000003.</notice>",
    ]);
    assert_eq!(incoming_view(entry.clone().into()), literal);
    assert_eq!(in_target_turn(entry.into()), literal);
}

#[test]
fn given_a_task_completed_without_summary_when_rendered_then_only_the_first_sentence() {
    let literal = strs(&["<notice>Task 01890a5d-ac96-774b-bcce-f00000000002 completed.</notice>"]);
    assert_eq!(incoming_view(completed().into()), literal);
    assert_eq!(in_target_turn(completed().into()), literal);
}

#[test]
fn given_a_task_completed_with_a_summary_when_rendered_then_the_summary_is_escaped() {
    let entry = completed().with_summary("all <done> & \"fine\"").unwrap();
    let literal = strs(&[
        "<notice>Task 01890a5d-ac96-774b-bcce-f00000000002 completed. Summary: all &lt;done&gt; &amp; \"fine\"</notice>",
    ]);
    assert_eq!(incoming_view(entry.clone().into()), literal);
    assert_eq!(in_target_turn(entry.into()), literal);
}

#[test]
fn given_a_task_failed_when_rendered_then_the_reason_is_escaped() {
    let entry = failed("</notice> & boom");
    let literal = strs(&[
        "<notice>Task 01890a5d-ac96-774b-bcce-f00000000002 failed. Reason: &lt;/notice&gt; &amp; boom</notice>",
    ]);
    assert_eq!(incoming_view(entry.clone().into()), literal);
    assert_eq!(in_target_turn(entry.into()), literal);
}

fn closed_view(reason: ClosureReason) -> Vec<(char, Vec<String>)> {
    let fixture = Fixture::new();
    let turns = vec![
        human_turn_of("go"),
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![blocks_response(vec![text("a")]), closure(reason)],
        ),
    ];
    view(&render_ok(&fixture, turns, fixed_turn(9)))
}

fn closure_message(reason: ClosureReason) -> Option<Vec<String>> {
    let fixture = Fixture::new();
    let turns = vec![
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![blocks_response(vec![text("a")]), closure(reason)],
        ),
        human_turn_of("go"),
    ];
    let rendered = view(&render_ok(&fixture, turns, fixed_turn(9)));
    rendered
        .into_iter()
        .find(|(role, _)| *role == 'U')
        .map(|(_, parts)| parts)
}

#[test]
fn given_a_completed_closure_with_a_detail_when_rendered_then_nothing_is_rendered_for_it() {
    let reason = ClosureReason::completed()
        .with_detail("secret detail")
        .unwrap();
    assert_eq!(closed_view(reason.clone()).len(), 2);
    assert_eq!(closure_message(reason), Some(strs(&["go"])));
}

#[test]
fn given_a_cancelled_closure_when_rendered_then_the_sentence_and_the_optional_detail() {
    assert_eq!(
        closure_message(ClosureReason::cancelled()),
        Some(strs(&[
            "<notice>Your previous turn was cancelled.</notice>",
            "go"
        ]))
    );
    assert_eq!(
        closure_message(ClosureReason::cancelled().with_detail("a<b & c").unwrap()),
        Some(strs(&[
            "<notice>Your previous turn was cancelled. Detail: a&lt;b &amp; c</notice>",
            "go"
        ]))
    );
}

#[test]
fn given_an_interrupted_closure_when_rendered_then_the_sentence_and_the_optional_detail() {
    assert_eq!(
        closure_message(ClosureReason::interrupted()),
        Some(strs(&[
            "<notice>Your previous turn was interrupted.</notice>",
            "go"
        ]))
    );
    assert_eq!(
        closure_message(ClosureReason::interrupted().with_detail("why").unwrap()),
        Some(strs(&[
            "<notice>Your previous turn was interrupted. Detail: why</notice>",
            "go"
        ]))
    );
}

#[test]
fn given_a_budget_exhausted_closure_when_rendered_then_the_sentence_and_the_optional_detail() {
    assert_eq!(
        closure_message(ClosureReason::budget_exhausted()),
        Some(strs(&[
            "<notice>Your previous turn stopped because its budget was exhausted.</notice>",
            "go"
        ]))
    );
    assert_eq!(
        closure_message(
            ClosureReason::budget_exhausted()
                .with_detail("10 steps")
                .unwrap()
        ),
        Some(strs(&[
            "<notice>Your previous turn stopped because its budget was exhausted. Detail: 10 steps</notice>",
            "go"
        ]))
    );
}

#[test]
fn given_a_failed_closure_when_rendered_then_the_detail_is_always_present_and_escaped() {
    assert_eq!(
        closure_message(ClosureReason::failed("tool <x> & y").unwrap()),
        Some(strs(&[
            "<notice>Your previous turn failed. Detail: tool &lt;x&gt; &amp; y</notice>",
            "go"
        ]))
    );
}
