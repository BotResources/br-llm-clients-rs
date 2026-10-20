use br_llm_clients_common_http::Secret;

use crate::text_clean::{Secrets, TEXT_CAP, cut};

fn secrets(values: &[&str]) -> Secrets {
    Secrets::new(
        values
            .iter()
            .map(|value| Secret::new(*value).expect("secret"))
            .collect(),
    )
}

#[test]
fn given_a_secret_inside_a_text_when_replaced_then_every_occurrence_becomes_the_placeholder() {
    let cleaned = secrets(&["sk-1"]).replace("a sk-1 b sk-1 c");
    assert_eq!(cleaned, "a [redacted] b [redacted] c");
}

#[test]
fn given_several_secrets_when_replaced_then_each_one_is_removed() {
    let cleaned = secrets(&["alpha", "beta", "gamma"]).replace("alpha beta gamma delta");
    assert_eq!(cleaned, "[redacted] [redacted] [redacted] delta");
}

#[test]
fn given_a_secret_that_holds_another_when_replaced_then_the_longest_goes_first() {
    for order in [["abc", "abcdef"], ["abcdef", "abc"]] {
        assert_eq!(secrets(&order).replace("xabcdefx"), "x[redacted]x");
    }
}

#[test]
fn given_no_secret_when_replaced_then_the_text_is_unchanged() {
    assert_eq!(Secrets::new(Vec::new()).replace("plain text"), "plain text");
}

#[test]
fn given_a_text_over_the_cap_when_cut_then_it_keeps_exactly_the_cap() {
    let text = "a".repeat(TEXT_CAP + 1);
    assert_eq!(cut(text), "a".repeat(65_536));
}

#[test]
fn given_a_text_at_the_cap_when_cut_then_it_is_unchanged() {
    let text = "a".repeat(65_536);
    assert_eq!(cut(text.clone()), text);
}

#[test]
fn given_a_cap_inside_a_character_when_cut_then_it_stops_at_the_boundary_before() {
    assert_eq!(cut("€".repeat(30_000)), "€".repeat(21_845));
    assert_eq!(cut("😀".repeat(20_000)), "😀".repeat(16_384));
}

#[test]
fn given_a_replacement_that_grows_the_text_when_cleaned_then_the_cut_applies_after_it() {
    let cleaned = secrets(&["S"]).clean(&"S".repeat(65_536));
    assert_eq!(cleaned.len(), 65_536);
    assert!(cleaned.starts_with("[redacted][redacted]"));
}
