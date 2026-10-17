use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::retry_after::parse;

const NOV_6_1994_084937: u64 = 784_111_777;

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

fn secs(value: &str) -> Option<Duration> {
    parse(value.as_bytes(), at(0))
}

#[test]
fn given_decimal_digits_when_parsing_then_they_are_seconds() {
    assert_eq!(secs("120"), Some(Duration::from_secs(120)));
    assert_eq!(secs("0"), Some(Duration::ZERO));
    assert_eq!(secs("007"), Some(Duration::from_secs(7)));
    assert_eq!(secs(" 5\t"), Some(Duration::from_secs(5)));
}

#[test]
fn given_a_number_too_large_when_parsing_then_it_saturates_at_the_largest_duration() {
    assert_eq!(
        secs("99999999999999999999999999999999"),
        Some(Duration::MAX)
    );
    assert_eq!(secs("18446744073709551616"), Some(Duration::MAX));
    assert_eq!(
        secs("18446744073709551615"),
        Some(Duration::from_secs(u64::MAX))
    );
}

#[test]
fn given_an_imf_fixdate_in_the_future_when_parsing_then_the_wait_is_the_difference() {
    let now = at(NOV_6_1994_084937 - 100);
    assert_eq!(
        parse(b"Sun, 06 Nov 1994 08:49:37 GMT", now),
        Some(Duration::from_secs(100))
    );
}

#[test]
fn given_an_rfc850_date_in_the_future_when_parsing_then_the_wait_is_the_difference() {
    let now = at(NOV_6_1994_084937 - 60);
    assert_eq!(
        parse(b"Sunday, 06-Nov-94 08:49:37 GMT", now),
        Some(Duration::from_secs(60))
    );
}

#[test]
fn given_an_asctime_date_in_the_future_when_parsing_then_the_wait_is_the_difference() {
    let now = at(NOV_6_1994_084937 - 30);
    assert_eq!(
        parse(b"Sun Nov  6 08:49:37 1994", now),
        Some(Duration::from_secs(30))
    );
}

#[test]
fn given_a_date_in_the_past_when_parsing_then_the_wait_is_zero() {
    let now = at(NOV_6_1994_084937 + 5);
    for date in [
        "Sun, 06 Nov 1994 08:49:37 GMT",
        "Sunday, 06-Nov-94 08:49:37 GMT",
        "Sun Nov  6 08:49:37 1994",
    ] {
        assert_eq!(parse(date.as_bytes(), now), Some(Duration::ZERO), "{date}");
    }
}

#[test]
fn given_the_current_second_when_parsing_a_date_then_the_wait_is_zero() {
    assert_eq!(
        parse(b"Sun, 06 Nov 1994 08:49:37 GMT", at(NOV_6_1994_084937)),
        Some(Duration::ZERO)
    );
}

#[test]
fn given_anything_else_when_parsing_then_there_is_nothing() {
    for value in [
        "",
        " ",
        "soon",
        "-5",
        "+5",
        "1.5",
        "5 seconds",
        "1e3",
        "0x10",
        "Sun, 32 Nov 1994 08:49:37 GMT",
        "Sun, 06 Nov 1994 08:49:37 PST",
        "5,6",
    ] {
        assert_eq!(secs(value), None, "{value:?}");
    }
    assert_eq!(parse(&[0xff, 0xfe], at(0)), None);
}
