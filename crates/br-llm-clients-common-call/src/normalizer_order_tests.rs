use crate::test_normalize::{fed, fresh, steps};

#[test]
fn given_a_later_block_when_an_earlier_one_is_open_then_its_events_wait_for_the_close() {
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
            vec!["start 0 Text".to_owned(), "text 0 \"A\"".to_owned()],
            vec![],
            vec![
                "end 0".to_owned(),
                "start 1 Thinking".to_owned(),
                "think 1 \"x\"".to_owned(),
                "end 1".to_owned()
            ],
        ]
    );
}

#[test]
fn given_the_first_open_block_when_it_gets_events_then_they_go_on_at_once() {
    let mut normalizer = fresh();
    let out = steps(
        &mut normalizer,
        &["start a thinking", "think a one", "sig a s1", "end a"],
    );
    assert_eq!(
        out,
        vec![
            vec!["start 0 Thinking".to_owned()],
            vec!["think 0 \"one\"".to_owned()],
            vec!["sig 0 \"s1\"".to_owned()],
            vec!["end 0".to_owned()],
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
        "redact c zz",
        "struct b {}",
        "end c",
        "end b",
        "text a hi",
        "end a",
    ]);
    assert_eq!(
        out,
        vec![
            "start 0 Text",
            "text 0 \"hi\"",
            "end 0",
            "start 1 Structured",
            "struct 1 \"{}\"",
            "end 1",
            "start 2 RedactedThinking",
            "redact 2 \"zz\"",
            "end 2",
        ]
    );
}

#[test]
fn given_a_closed_key_when_it_starts_again_then_it_is_a_new_block_with_the_next_index() {
    let out = fed(&[
        "start a thinking",
        "end a",
        "start a thinking",
        "think a again",
        "end a",
    ]);
    assert_eq!(
        out,
        vec![
            "start 0 Thinking",
            "end 0",
            "start 1 Thinking",
            "think 1 \"again\"",
            "end 1"
        ]
    );
}

#[test]
fn given_a_tool_call_block_when_it_starts_then_the_start_event_carries_its_id_and_name() {
    let out = fed(&[
        "start t tool search call_9",
        "args t {\"q\"",
        "args t :1}",
        "end t",
    ]);
    assert_eq!(
        out,
        vec![
            "call 0 call_9 search",
            "args 0 \"{\\\"q\\\"\"",
            "args 0 \":1}\"",
            "end 0"
        ]
    );
}

#[test]
fn given_every_block_kind_when_started_then_the_start_event_names_that_kind() {
    let out = fed(&[
        "start a text",
        "text a x",
        "end a",
        "start b thinking",
        "end b",
        "start c redacted",
        "end c",
        "start d structured",
        "end d",
    ]);
    assert_eq!(
        out,
        vec![
            "start 0 Text",
            "text 0 \"x\"",
            "end 0",
            "start 1 Thinking",
            "end 1",
            "start 2 RedactedThinking",
            "end 2",
            "start 3 Structured",
            "end 3",
        ]
    );
}
