use crate::test_normalize::{fed, fresh, steps};

#[test]
fn given_a_later_block_when_an_earlier_one_is_open_then_its_deltas_wait_for_the_close() {
    let mut normalizer = fresh();
    let out = steps(
        &mut normalizer,
        &[
            "start a text",
            "start b thinking",
            "think b x",
            "text a A",
            "end b",
            "end a",
        ],
    );
    assert_eq!(
        out,
        vec![
            vec![],
            vec![],
            vec![],
            vec!["start 1 text".to_owned(), "text 1 \"A\"".to_owned()],
            vec![],
            vec![
                "final 1".to_owned(),
                "start 2 thinking".to_owned(),
                "think 2 \"x\"".to_owned(),
                "final 2".to_owned()
            ],
        ]
    );
}

#[test]
fn given_the_first_open_block_when_it_gets_deltas_then_they_go_on_at_once() {
    let mut normalizer = fresh();
    let out = steps(
        &mut normalizer,
        &["start a thinking", "think a one", "sig a fmt/1 s1", "end a"],
    );
    assert_eq!(
        out,
        vec![
            vec![],
            vec!["start 1 thinking".to_owned(), "think 1 \"one\"".to_owned()],
            vec!["replay 1 fmt/1 \"s1\"".to_owned()],
            vec!["final 1".to_owned()],
        ]
    );
}

#[test]
fn given_blocks_that_start_in_one_order_when_the_later_one_closes_first_then_the_start_order_stands()
 {
    let out = fed(&[
        "start a text",
        "start b structured",
        "start c redacted",
        "redact c fmt/1 zz",
        "struct b {}",
        "end c",
        "end b",
        "text a hi",
        "end a",
    ]);
    assert_eq!(
        out,
        vec![
            "start 1 text",
            "text 1 \"hi\"",
            "final 1",
            "start 2 structured",
            "json 2 \"{}\"",
            "final 2",
            "start 3 redacted",
            "replay 3 fmt/1 \"zz\"",
            "final 3",
        ]
    );
}

#[test]
fn given_a_closed_key_when_it_starts_again_then_it_is_a_new_block_with_a_new_id() {
    let out = fed(&[
        "start a thinking",
        "think a first",
        "end a",
        "start a thinking",
        "think a again",
        "end a",
    ]);
    assert_eq!(
        out,
        vec![
            "start 1 thinking",
            "think 1 \"first\"",
            "final 1",
            "start 2 thinking",
            "think 2 \"again\"",
            "final 2"
        ]
    );
}

#[test]
fn given_a_tool_call_block_when_it_starts_then_the_start_carries_its_name_and_external_id() {
    let out = fed(&[
        "start t tool search call_9",
        "args t {\"q\"",
        "args t :1}",
        "end t",
    ]);
    assert_eq!(
        out,
        vec![
            "start 1 call search call_9",
            "json 1 \"{\\\"q\\\"\"",
            "json 1 \":1}\"",
            "final 1"
        ]
    );
}

#[test]
fn given_every_block_kind_when_started_then_the_start_names_that_kind() {
    let out = fed(&[
        "start a text",
        "text a x",
        "end a",
        "start b thinking",
        "think b y",
        "end b",
        "start c redacted",
        "end c",
        "start d structured",
        "end d",
    ]);
    assert_eq!(
        out,
        vec![
            "start 1 text",
            "text 1 \"x\"",
            "final 1",
            "start 2 thinking",
            "think 2 \"y\"",
            "final 2",
            "start 3 redacted",
            "final 3",
            "start 4 structured",
        ]
    );
}
