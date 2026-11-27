use serde_json::{Value, json};

use crate::{CacheTtl, MessagesCacheOptions, OptionsError};

fn refused(value: Value) -> OptionsError {
    MessagesCacheOptions::from_json(&value).expect_err("refused")
}

fn invalid(member: Option<&str>) -> OptionsError {
    OptionsError::InvalidCacheOption {
        member: member.map(str::to_owned),
    }
}

#[test]
fn given_the_default_cache_options_when_read_then_the_ttl_is_five_minutes() {
    assert_eq!(MessagesCacheOptions::default().ttl(), CacheTtl::FiveMinutes);
}

#[test]
fn given_an_empty_object_when_read_from_json_then_it_gives_the_default() {
    let options = MessagesCacheOptions::from_json(&json!({})).expect("accepted");
    assert_eq!(options, MessagesCacheOptions::default());
}

#[test]
fn given_each_written_ttl_when_read_from_json_then_it_gives_that_ttl() {
    for (text, ttl) in [("5m", CacheTtl::FiveMinutes), ("1h", CacheTtl::OneHour)] {
        let options = MessagesCacheOptions::from_json(&json!({"ttl": text})).expect("accepted");
        assert_eq!(options.ttl(), ttl, "{text}");
    }
}

#[test]
fn given_a_value_that_is_not_an_object_when_read_from_json_then_it_is_refused_with_no_member() {
    for value in [
        json!(null),
        json!("1h"),
        json!(1),
        json!(true),
        json!([]),
        json!([{"ttl": "1h"}]),
    ] {
        assert_eq!(refused(value.clone()), invalid(None), "{value}");
    }
}

#[test]
fn given_a_member_other_than_ttl_when_read_from_json_then_it_is_refused_with_that_member() {
    for (value, member) in [
        (json!({"lifetime": "1h"}), "lifetime"),
        (json!({"TTL": "1h"}), "TTL"),
        (json!({"ttl": "1h", "extra": 1}), "extra"),
        (json!({"type": "ephemeral"}), "type"),
    ] {
        assert_eq!(refused(value.clone()), invalid(Some(member)), "{value}");
    }
}

#[test]
fn given_a_ttl_that_is_not_one_of_the_two_texts_when_read_from_json_then_it_is_refused_with_ttl() {
    for ttl in [
        json!("2h"),
        json!("1H"),
        json!(" 1h"),
        json!(""),
        json!(null),
        json!(3600),
        json!(true),
        json!(["1h"]),
        json!({"value": "1h"}),
    ] {
        assert_eq!(
            refused(json!({ "ttl": ttl.clone() })),
            invalid(Some("ttl")),
            "{ttl}"
        );
    }
}

#[test]
fn given_a_refusal_when_shown_then_the_text_names_the_member_and_shows_no_value() {
    let text = refused(json!({"ttl": "secret-value-3"})).to_string();
    assert!(text.contains("`ttl`"), "{text}");
    assert!(!text.contains("secret-value-3"), "{text}");
    let text = refused(json!({"unknown": "secret-value-4"})).to_string();
    assert!(text.contains("`unknown`"), "{text}");
    assert!(!text.contains("secret-value-4"), "{text}");
    let text = refused(json!("secret-value-5")).to_string();
    assert!(!text.contains("secret-value-5"), "{text}");
}

#[test]
fn given_cache_options_when_copied_then_the_copy_is_equal() {
    let options = MessagesCacheOptions::from_json(&json!({"ttl": "1h"})).expect("accepted");
    let copy = options;
    assert_eq!(options, copy);
    assert!(format!("{copy:?}").contains("OneHour"));
}
