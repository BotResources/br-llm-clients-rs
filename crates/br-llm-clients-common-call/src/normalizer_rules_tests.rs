use br_llm_messages::{ModelName, ResponseEnding, Usage};

use crate::test_decoder::script;
use crate::test_fixtures::user_request;
use crate::test_ids::CountingIds;
use crate::test_normalize::{feed, fresh, is_malformed, refusal};
use crate::test_stream::kinds;

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

fn delta_line(delta: &str) -> String {
    match delta {
        "sig" | "redact" => format!("{delta} a fmt/1 x"),
        other => format!("{other} a x"),
    }
}

#[test]
fn given_a_delta_that_does_not_fit_its_block_when_accepted_then_the_reply_is_malformed() {
    for (start, own) in STARTS {
        for delta in DELTAS {
            let mut normalizer = fresh();
            feed(&mut normalizer, &[start]);
            let line = delta_line(delta);
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
        "sig a fmt/1 x",
        "struct a x",
        "args a x",
        "redact a fmt/1 x",
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
    let setup = ["start a text", "stop finished"];
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
    assert!(is_malformed(&refusal(&["stop finished"], "stop exhaust")));
}

#[test]
fn given_stop_usage_model_and_response_id_in_any_order_when_accepted_then_the_last_of_each_stands_and_each_is_sent()
 {
    let mut normalizer = fresh();
    let out = feed(
        &mut normalizer,
        &[
            "usage 1 2",
            "model first",
            "rid r1",
            "stop exhaust",
            "usage 5 6",
            "model second",
            "rid r2",
        ],
    );
    assert_eq!(
        kinds(&out),
        vec![
            "started",
            "meta usage=Some(1)/Some(2)",
            "meta model=first",
            "meta rid=r1",
            "meta usage=Some(5)/Some(6)",
            "meta model=second",
            "meta rid=r2",
        ]
    );
    assert_eq!(
        normalizer.stop(),
        Some(&ResponseEnding::output_exhaustion())
    );
    assert_eq!(
        normalizer.usage(),
        Some(&Usage::of_input_tokens(5).with_output_tokens(6))
    );
    assert_eq!(
        normalizer.model(),
        Some(&ModelName::new("second").expect("model"))
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
fn given_an_error_or_complete_event_when_accepted_then_nothing_is_sent_not_even_the_start() {
    let mut normalizer = fresh();
    let out = feed(&mut normalizer, &["error 500 - - boom", "complete"]);
    assert!(out.is_empty());
    assert_eq!(normalizer.stop(), None);
    assert!(normalizer.held().is_empty());
}

#[test]
fn given_a_tool_name_that_is_not_valid_when_started_then_the_reply_is_malformed() {
    for name in ["bad.name", "9start", "has_space!", "x".repeat(257).as_str()] {
        let line = format!("start a tool {name} id1");
        assert!(is_malformed(&refusal(&[], &line)), "{name}");
    }
    let mut normalizer = fresh();
    assert!(!feed(&mut normalizer, &["start a tool good_name-1 id1"]).is_empty());
}

#[test]
fn given_the_first_accepted_event_when_it_comes_then_the_start_is_sent_before_everything_else() {
    for line in [
        "start a text",
        "usage 1",
        "model m",
        "rid r",
        "stop finished",
    ] {
        let out = kinds(&feed(&mut fresh(), &[line]));
        assert_eq!(out.first().map(String::as_str), Some("started"), "{line}");
        assert_eq!(
            out.iter()
                .filter(|line| line.starts_with("started"))
                .count(),
            1
        );
    }
}

#[test]
fn given_a_later_event_when_it_comes_then_the_start_is_not_sent_again() {
    let mut normalizer = fresh();
    feed(&mut normalizer, &["usage 1"]);
    let out = kinds(&feed(
        &mut normalizer,
        &["model m", "start a text", "stop finished"],
    ));
    assert!(
        out.iter().all(|line| !line.starts_with("started")),
        "{out:?}"
    );
}

#[test]
fn given_a_first_event_that_is_refused_when_accepted_then_no_start_is_sent_and_the_next_one_sends_it()
 {
    let mut normalizer = fresh();
    for line in ["text a x", "end a", "start a tool bad.name", "model "] {
        let event = script(&[line]).remove(0);
        assert!(normalizer.accept(event).is_err(), "{line}");
    }
    let out = kinds(&feed(&mut normalizer, &["usage 1"]));
    assert_eq!(out.first().map(String::as_str), Some("started"));
}

#[test]
fn given_an_answered_model_when_the_start_is_sent_then_it_holds_the_model() {
    let mut normalizer = crate::normalizer::Normalizer::new(
        &user_request(),
        std::sync::Arc::new(CountingIds::new()),
        Some(ModelName::new("gw").expect("model")),
    );
    let out = kinds(
        &script(&["usage 1"])
            .into_iter()
            .flat_map(|event| normalizer.accept(event).expect("accepted"))
            .collect::<Vec<_>>(),
    );
    assert_eq!(out.first().map(String::as_str), Some("started gw"));
}

#[test]
fn given_a_usage_event_that_holds_one_count_when_accepted_then_it_is_sent_as_an_update() {
    let mut normalizer = fresh();
    let out = kinds(&feed(&mut normalizer, &["usage 7"]));
    assert_eq!(out, vec!["started", "meta usage=Some(7)/None"]);
    assert_eq!(normalizer.usage(), Some(&Usage::of_input_tokens(7)));
}

#[test]
fn given_an_empty_response_id_when_accepted_then_nothing_changes_and_an_earlier_id_stands() {
    let mut normalizer = fresh();
    let out = kinds(&feed(&mut normalizer, &["rid r1", "rid ", "rid r2"]));
    assert_eq!(out, vec!["started", "meta rid=r1", "meta rid=r2"]);
}
