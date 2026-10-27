use crate::judge::{Held, Json};
use crate::test_normalize::{feed, fresh};
use crate::test_stream::kinds;

#[test]
fn given_open_blocks_when_closed_at_the_end_then_they_close_in_the_order_of_their_start() {
    let mut normalizer = fresh();
    feed(
        &mut normalizer,
        &[
            "start a thinking",
            "think a t",
            "start b text",
            "text b Hi",
            "start c redacted",
            "redact c fmt/1 zz",
        ],
    );
    assert_eq!(
        kinds(&normalizer.close_all()),
        vec![
            "final 1",
            "start 2 text",
            "text 2 \"Hi\"",
            "final 2",
            "start 3 redacted",
            "replay 3 fmt/1 \"zz\"",
            "final 3",
        ]
    );
}

#[test]
fn given_an_open_text_block_that_is_still_blank_when_closed_then_it_is_dropped_with_its_held_deltas()
 {
    let mut normalizer = fresh();
    feed(&mut normalizer, &["start a text", "text a  ", "text a "]);
    assert!(normalizer.close_all().is_empty());
}

#[test]
fn given_an_open_thinking_block_that_holds_nothing_when_closed_then_it_is_dropped() {
    let mut normalizer = fresh();
    feed(&mut normalizer, &["start a thinking", "think a "]);
    assert!(normalizer.close_all().is_empty());
}

#[test]
fn given_a_blank_block_before_a_real_one_when_closed_then_the_real_one_is_the_first_sent() {
    let mut normalizer = fresh();
    feed(
        &mut normalizer,
        &["start a text", "text a  ", "start b thinking", "think b t"],
    );
    assert_eq!(
        kinds(&normalizer.close_all()),
        vec!["start 1 thinking", "think 1 \"t\"", "final 1"]
    );
}

#[test]
fn given_blocks_closed_by_the_provider_when_closed_at_the_end_then_no_delta_is_added() {
    let mut normalizer = fresh();
    feed(&mut normalizer, &["start a thinking", "think a t", "end a"]);
    assert!(normalizer.close_all().is_empty());
}

#[test]
fn given_no_block_when_closed_then_nothing_is_sent_and_a_second_close_sends_nothing() {
    let mut normalizer = fresh();
    assert!(normalizer.close_all().is_empty());
    feed(&mut normalizer, &["start a thinking", "think a t"]);
    assert!(!normalizer.close_all().is_empty());
    assert!(normalizer.close_all().is_empty());
}

fn held_json(lines: &[&str]) -> Held {
    let mut normalizer = fresh();
    feed(&mut normalizer, lines);
    normalizer.held().remove(0)
}

fn tool(args: &[&str], close: bool) -> Held {
    let mut lines = vec!["start a tool search id1".to_owned()];
    lines.extend(args.iter().map(|part| format!("args a {part}")));
    if close {
        lines.push("end a".to_owned());
    }
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    held_json(&refs)
}

#[test]
fn given_json_closed_by_the_provider_when_judged_then_a_complete_object_is_complete() {
    for json in ["{}", "{\"a\":1}", " {} "] {
        assert_eq!(
            tool(&[json], true),
            Held::ToolCall(Json::Complete),
            "{json}"
        );
    }
    assert_eq!(
        tool(&["{\"a\"", ":1}"], true),
        Held::ToolCall(Json::Complete)
    );
}

#[test]
fn given_arguments_that_are_complete_json_but_not_an_object_when_judged_then_they_are_malformed() {
    for json in ["[1,2]", "\"x\"", "7", "null"] {
        assert!(
            matches!(tool(&[json], true), Held::ToolCall(Json::Malformed(_))),
            "{json}"
        );
    }
}

#[test]
fn given_json_that_ends_too_soon_when_judged_then_it_is_incomplete() {
    for json in ["{\"a\":", "{\"a\":1", "[1,", "tru", "\"abc", "   ", "{"] {
        assert_eq!(
            tool(&[json], true),
            Held::ToolCall(Json::Incomplete),
            "{json}"
        );
    }
}

#[test]
fn given_empty_json_when_judged_then_it_is_incomplete() {
    assert_eq!(tool(&[], true), Held::ToolCall(Json::Incomplete));
}

#[test]
fn given_a_block_the_provider_did_not_close_when_judged_then_its_json_is_incomplete_even_if_valid()
{
    assert_eq!(tool(&["{}"], false), Held::ToolCall(Json::Incomplete));
    let mut normalizer = fresh();
    feed(&mut normalizer, &["start a tool search id1", "args a {}"]);
    normalizer.close_all();
    assert_eq!(normalizer.held(), vec![Held::ToolCall(Json::Incomplete)]);
}

#[test]
fn given_json_that_is_wrong_before_its_end_when_judged_then_it_is_malformed_not_incomplete() {
    for json in ["{]", "{\"a\":1}}", "nul1", "{} x", "[1 2]"] {
        assert!(
            matches!(tool(&[json], true), Held::ToolCall(Json::Malformed(_))),
            "{json}"
        );
    }
}

#[test]
fn given_json_the_draft_would_refuse_when_judged_then_it_is_malformed() {
    assert!(matches!(
        tool(&["{\"a\":1,\"a\":2}"], true),
        Held::ToolCall(Json::Malformed(_))
    ));
}

#[test]
fn given_a_structured_block_when_judged_then_it_follows_the_same_json_rules() {
    assert_eq!(
        held_json(&["start a structured", "struct a {\"k\":1}", "end a"]),
        Held::Structured(Json::Complete)
    );
    assert_eq!(
        held_json(&["start a structured", "struct a [1]", "end a"]),
        Held::Structured(Json::Complete)
    );
    assert_eq!(
        held_json(&["start a structured", "struct a {\"k\":", "end a"]),
        Held::Structured(Json::Incomplete)
    );
    assert_eq!(
        held_json(&["start a structured", "end a"]),
        Held::Structured(Json::Incomplete)
    );
    assert!(matches!(
        held_json(&["start a structured", "struct a ]", "end a"]),
        Held::Structured(Json::Malformed(_))
    ));
}

#[test]
fn given_the_other_blocks_when_judged_then_a_text_block_tells_if_it_holds_a_real_character() {
    assert_eq!(
        held_json(&["start a text", "text a  ", "text a x", "end a"]),
        Held::Text { nonblank: true }
    );
    assert_eq!(
        held_json(&["start a text", "text a  ", "end a"]),
        Held::Text { nonblank: false }
    );
    assert_eq!(held_json(&["start a thinking"]), Held::Thinking);
    assert_eq!(held_json(&["start a redacted"]), Held::Redacted);
}

#[test]
fn given_a_complete_json_block_when_the_provider_closes_it_then_it_is_finalized_at_once() {
    let mut normalizer = fresh();
    feed(&mut normalizer, &["start a tool search id1", "args a {}"]);
    assert_eq!(kinds(&feed(&mut normalizer, &["end a"])), vec!["final 1"]);
}

#[test]
fn given_an_incomplete_json_block_when_the_provider_closes_it_then_no_finalization_is_sent_and_the_next_block_goes_on()
 {
    let mut normalizer = fresh();
    feed(
        &mut normalizer,
        &["start a tool search id1", "args a {\"q\":", "start b text"],
    );
    assert_eq!(
        kinds(&feed(&mut normalizer, &["end a", "text b Hi"])),
        vec!["start 2 text", "text 2 \"Hi\""]
    );
}
