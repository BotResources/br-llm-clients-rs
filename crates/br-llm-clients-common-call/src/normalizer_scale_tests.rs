use crate::test_decoder::script;
use crate::test_normalize::{feed, fresh};
use crate::test_stream::kinds;

const BLOCKS: usize = 20_000;

fn block_lines(count: usize) -> Vec<String> {
    (0..count)
        .flat_map(|number| {
            [
                format!("start k{number} text"),
                format!("text k{number} hi"),
                format!("end k{number}"),
            ]
        })
        .collect()
}

fn as_refs(lines: &[String]) -> Vec<&str> {
    lines.iter().map(String::as_str).collect()
}

#[test]
fn given_many_blocks_that_start_and_end_in_turn_when_accepted_then_none_is_kept_and_all_are_sent() {
    let mut normalizer = fresh();
    let lines = block_lines(BLOCKS);
    let out = feed(&mut normalizer, &as_refs(&lines));
    assert_eq!(out.len(), 1 + BLOCKS * 3);
    assert_eq!(normalizer.pending_blocks(), 0);
    assert_eq!(normalizer.held().len(), BLOCKS);
}

#[test]
fn given_many_blocks_behind_an_open_head_when_the_head_ends_then_all_are_sent_in_order_and_none_is_kept()
 {
    let mut normalizer = fresh();
    let mut lines = vec!["start head text".to_owned(), "text head first".to_owned()];
    lines.extend(block_lines(BLOCKS));
    let before = feed(&mut normalizer, &as_refs(&lines));
    assert_eq!(kinds(&before).len(), 3);
    assert_eq!(normalizer.pending_blocks(), BLOCKS + 1);
    let released: Vec<_> = script(&["end head"])
        .into_iter()
        .flat_map(|event| normalizer.accept(event).expect("accepted"))
        .collect();
    assert_eq!(released.len(), 1 + BLOCKS * 3);
    assert_eq!(normalizer.pending_blocks(), 0);
    assert_eq!(normalizer.held().len(), BLOCKS + 1);
}

#[test]
fn given_a_key_reused_after_its_block_ended_when_accepted_then_the_new_block_is_found() {
    let mut normalizer = fresh();
    let out = kinds(&feed(
        &mut normalizer,
        &[
            "start head text",
            "text head a",
            "start k text",
            "end k",
            "start k text",
            "text k second",
            "end head",
            "end k",
        ],
    ));
    assert_eq!(
        out,
        vec![
            "started",
            "start 1 text",
            "text 1 \"a\"",
            "final 1",
            "start 2 text",
            "text 2 \"second\"",
            "final 2"
        ]
    );
}
