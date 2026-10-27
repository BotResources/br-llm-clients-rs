use crate::test_normalize::{fed, fresh, steps};

#[test]
fn given_blank_deltas_when_a_real_one_comes_then_the_start_and_the_blank_ones_go_just_before_it() {
    let mut normalizer = fresh();
    let out = steps(
        &mut normalizer,
        &[
            "start a text",
            "text a ",
            "text a  \t",
            "text a Hi",
            "text a  there",
        ],
    );
    assert_eq!(
        out,
        vec![
            vec![],
            vec![],
            vec![],
            vec![
                "start 1 text".to_owned(),
                "text 1 \"\"".to_owned(),
                "text 1 \" \\t\"".to_owned(),
                "text 1 \"Hi\"".to_owned()
            ],
            vec!["text 1 \" there\"".to_owned()],
        ]
    );
}

#[test]
fn given_a_blank_delta_after_real_text_when_sent_then_it_is_not_held() {
    let out = fed(&["start a text", "text a Hi", "text a  ", "text a "]);
    assert_eq!(
        out,
        vec![
            "start 1 text",
            "text 1 \"Hi\"",
            "text 1 \" \"",
            "text 1 \"\""
        ]
    );
}

#[test]
fn given_a_text_block_blank_from_start_to_end_when_closed_then_no_delta_is_sent_for_it() {
    let out = fed(&["start a text", "text a  ", "text a ", "end a"]);
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn given_a_dropped_blank_block_when_the_next_block_starts_then_the_ids_come_from_the_blocks_sent() {
    let out = fed(&[
        "start a text",
        "text a   ",
        "end a",
        "start b thinking",
        "end b",
        "start c text",
        "end c",
        "start d text",
        "text d Real",
        "end d",
    ]);
    assert_eq!(out, vec!["start 1 text", "text 1 \"Real\"", "final 1"]);
}

#[test]
fn given_a_held_text_block_when_it_is_released_then_the_holdback_still_applies_to_it() {
    let mut normalizer = fresh();
    let out = steps(
        &mut normalizer,
        &[
            "start a tool search id1",
            "start b text",
            "text b  ",
            "text b Go",
            "end b",
            "args a {}",
            "end a",
        ],
    );
    assert_eq!(
        out.last().cloned().unwrap_or_default(),
        vec![
            "final 1".to_owned(),
            "start 2 text".to_owned(),
            "text 2 \" \"".to_owned(),
            "text 2 \"Go\"".to_owned(),
            "final 2".to_owned()
        ]
    );
}

#[test]
fn given_a_held_blank_text_block_when_released_then_nothing_is_sent_for_it() {
    let mut normalizer = fresh();
    let out = steps(
        &mut normalizer,
        &[
            "start a tool search id1",
            "start b text",
            "text b  ",
            "end b",
            "args a {}",
            "end a",
        ],
    );
    assert_eq!(
        out.last().cloned().unwrap_or_default(),
        vec!["final 1".to_owned()]
    );
}

#[test]
fn given_a_text_block_that_never_got_real_text_when_the_next_block_waits_then_it_waits_until_the_close()
 {
    let mut normalizer = fresh();
    let out = steps(
        &mut normalizer,
        &["start a text", "text a  ", "start b thinking", "think b t"],
    );
    assert!(out.iter().all(Vec::is_empty), "{out:?}");
    let released = steps(&mut normalizer, &["end a"]);
    assert_eq!(
        released,
        vec![vec![
            "start 1 thinking".to_owned(),
            "think 1 \"t\"".to_owned()
        ]]
    );
}
