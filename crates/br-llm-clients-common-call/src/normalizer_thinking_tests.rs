use crate::test_normalize::{fed, fresh, steps};

#[test]
fn given_empty_thinking_deltas_when_a_text_comes_then_the_start_and_the_empty_ones_go_just_before_it()
 {
    let mut normalizer = fresh();
    let out = steps(
        &mut normalizer,
        &[
            "start a thinking",
            "think a ",
            "sig a fmt/1 ",
            "think a hmm",
            "think a  more",
        ],
    );
    assert_eq!(
        out,
        vec![
            vec![],
            vec![],
            vec![],
            vec![
                "start 1 thinking".to_owned(),
                "think 1 \"\"".to_owned(),
                "replay 1 fmt/1 \"\"".to_owned(),
                "think 1 \"hmm\"".to_owned()
            ],
            vec!["think 1 \" more\"".to_owned()],
        ]
    );
}

#[test]
fn given_a_non_empty_replay_fragment_when_it_comes_first_then_it_releases_the_thinking_block() {
    let out = fed(&[
        "start a thinking",
        "think a ",
        "sig a fmt/1 s1",
        "sig a fmt/1 s2",
        "end a",
    ]);
    assert_eq!(
        out,
        vec![
            "start 1 thinking",
            "think 1 \"\"",
            "replay 1 fmt/1 \"s1\"",
            "replay 1 fmt/1 \"s2\"",
            "final 1"
        ]
    );
}

#[test]
fn given_a_thinking_block_with_only_empty_deltas_when_closed_then_nothing_is_sent_for_it() {
    let out = fed(&[
        "start a thinking",
        "think a ",
        "sig a fmt/1 ",
        "end a",
        "start b thinking",
        "end b",
    ]);
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn given_a_blank_but_not_empty_thinking_delta_when_it_comes_then_it_releases_the_block() {
    let out = fed(&["start a thinking", "think a  ", "end a"]);
    assert_eq!(out, vec!["start 1 thinking", "think 1 \" \"", "final 1"]);
}

#[test]
fn given_an_empty_thinking_delta_after_the_block_started_then_it_is_sent() {
    let out = fed(&["start a thinking", "think a x", "think a ", "end a"]);
    assert_eq!(
        out,
        vec![
            "start 1 thinking",
            "think 1 \"x\"",
            "think 1 \"\"",
            "final 1"
        ]
    );
}

#[test]
fn given_a_redacted_thinking_block_then_it_is_not_held_back() {
    let out = fed(&["start a redacted", "redact a fmt/2 zz", "end a"]);
    assert_eq!(
        out,
        vec!["start 1 redacted", "replay 1 fmt/2 \"zz\"", "final 1"]
    );
}
