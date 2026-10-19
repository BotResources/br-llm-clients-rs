use br_llm_messages::{StopReason, Usage};

use crate::test_decoder::script;
use crate::test_normalize::{feed, fresh, is_malformed, refusal};

const STARTS: [(&str, &str); 5] = [
    ("start a text", "text"),
    ("start a thinking", "think"),
    ("start a redacted", "redact"),
    ("start a structured", "struct"),
    ("start a tool search id1", "args"),
];

const DELTAS: [&str; 6] = ["text", "think", "sig", "redact", "struct", "args"];

fn fits(start_delta: &str, delta: &str) -> bool {
    start_delta == delta || (start_delta == "think" && delta == "sig")
}

#[test]
fn given_a_delta_that_does_not_fit_its_block_when_accepted_then_the_reply_is_malformed() {
    for (start, own) in STARTS {
        for delta in DELTAS {
            let mut normalizer = fresh();
            feed(&mut normalizer, &[start]);
            let line = format!("{delta} a x");
            let event = script(&[line.as_str()]).remove(0);
            let result = normalizer.accept(event);
            if fits(own, delta) {
                assert!(result.is_ok(), "{start} {delta}");
            } else {
                assert!(
                    is_malformed(&result.expect_err("refused")),
                    "{start} {delta}"
                );
            }
        }
    }
}

#[test]
fn given_an_event_for_a_key_that_is_not_open_when_accepted_then_the_reply_is_malformed() {
    for line in [
        "text a x",
        "think a x",
        "sig a x",
        "struct a x",
        "args a x",
        "redact a x",
        "end a",
    ] {
        assert!(is_malformed(&refusal(&[], line)), "{line}");
        assert!(
            is_malformed(&refusal(&["start b text"], line)),
            "{line} on another key"
        );
        assert!(
            is_malformed(&refusal(&["start a text", "end a"], line)),
            "{line} after the close"
        );
    }
}

#[test]
fn given_a_start_for_a_key_that_is_open_when_accepted_then_the_reply_is_malformed() {
    let error = refusal(&["start a text"], "start a thinking");
    assert!(is_malformed(&error), "{error:?}");
}

#[test]
fn given_a_block_event_after_the_stop_when_accepted_then_the_reply_is_malformed() {
    let setup = ["start a text", "stop end_turn"];
    for line in [
        "text a x",
        "end a",
        "start b text",
        "start b tool search id2",
    ] {
        assert!(is_malformed(&refusal(&setup, line)), "{line}");
    }
}

#[test]
fn given_a_second_stop_when_accepted_then_the_reply_is_malformed() {
    assert!(is_malformed(&refusal(&["stop end_turn"], "stop max")));
}

#[test]
fn given_stop_usage_and_model_in_any_order_when_accepted_then_the_last_usage_and_model_stand() {
    let mut normalizer = fresh();
    let out = feed(
        &mut normalizer,
        &[
            "usage 1 2",
            "model first",
            "stop max",
            "usage 5 6",
            "model second",
        ],
    );
    assert!(out.is_empty());
    assert_eq!(normalizer.stop(), Some(&StopReason::MaxTokens));
    assert_eq!(
        normalizer.usage(),
        Some(&Usage {
            input_tokens: 5,
            output_tokens: 6,
            ..Usage::default()
        })
    );
    assert_eq!(
        normalizer.model().map(|model| model.as_str()),
        Some("second")
    );
}

#[test]
fn given_an_event_stream_with_no_stop_when_asked_then_no_stop_is_known() {
    let mut normalizer = fresh();
    feed(&mut normalizer, &["usage 1 2", "model m"]);
    assert_eq!(normalizer.stop(), None);
}

#[test]
fn given_a_model_that_is_not_valid_when_accepted_then_the_reply_is_malformed_and_the_old_model_stands()
 {
    let mut normalizer = fresh();
    feed(&mut normalizer, &["model good"]);
    let event = script(&["model "]).remove(0);
    let error = normalizer.accept(event).expect_err("refused");
    assert!(is_malformed(&error));
    assert_eq!(normalizer.model().map(|model| model.as_str()), Some("good"));
}

#[test]
fn given_an_error_or_complete_event_when_accepted_then_the_normalizer_does_nothing() {
    let mut normalizer = fresh();
    let out = feed(&mut normalizer, &["error 500 - - boom", "complete"]);
    assert!(out.is_empty());
    assert_eq!(normalizer.stop(), None);
    assert!(normalizer.held().is_empty());
}

#[test]
fn given_a_tool_name_that_is_not_valid_when_started_then_the_reply_is_malformed() {
    for name in ["bad.name", "9start", "has_space!", "x".repeat(65).as_str()] {
        let line = format!("start a tool {name} id1");
        assert!(is_malformed(&refusal(&[], &line)), "{name}");
    }
    let mut normalizer = fresh();
    assert!(!feed(&mut normalizer, &["start a tool good_name-1 id1"]).is_empty());
}
