use http::{HeaderMap, HeaderValue};

use crate::headers::Headers;

fn sample() -> Headers {
    let mut map = HeaderMap::new();
    map.append("X-Multi", HeaderValue::from_static("one"));
    map.append("content-type", HeaderValue::from_static("text/plain"));
    map.append("x-multi", HeaderValue::from_static("two"));
    map.append("x-multi", HeaderValue::from_static("three"));
    map.append(
        "x-bytes",
        HeaderValue::from_bytes(&[0xff, 0xfe]).expect("value"),
    );
    Headers::new(map)
}

#[test]
fn given_a_repeated_header_when_reading_all_then_the_values_keep_their_order() {
    let headers = sample();
    let all: Vec<&[u8]> = headers.get_all("x-multi").collect();
    assert_eq!(all, [b"one".as_slice(), b"two", b"three"]);
}

#[test]
fn given_any_case_when_looking_up_then_the_header_is_found() {
    let headers = sample();
    assert_eq!(headers.get("CONTENT-TYPE"), Some(b"text/plain".as_slice()));
    assert!(headers.contains("Content-Type"));
    assert_eq!(headers.get("x-multi"), Some(b"one".as_slice()));
}

#[test]
fn given_names_when_listing_then_they_are_in_lower_case() {
    let headers = sample();
    assert!(headers.names().all(|n| n == n.to_ascii_lowercase()));
    assert!(headers.iter().all(|(n, _)| n == n.to_ascii_lowercase()));
}

#[test]
fn given_a_value_when_reading_as_text_then_only_valid_utf8_comes_back() {
    let headers = sample();
    assert_eq!(headers.get_text("content-type"), Some("text/plain"));
    assert_eq!(headers.get_text("x-bytes"), None);
    assert_eq!(headers.get("x-bytes"), Some([0xff, 0xfe].as_slice()));
}

#[test]
fn given_a_missing_or_invalid_name_when_looking_up_then_there_is_nothing() {
    let headers = sample();
    assert_eq!(headers.get("absent"), None);
    assert_eq!(headers.get("bad name"), None);
    assert_eq!(headers.get_all("").count(), 0);
}

#[test]
fn given_headers_when_formatting_debug_then_values_are_not_shown() {
    let debug = format!("{:?}", sample());
    assert!(debug.contains("x-multi"));
    assert!(
        !debug.contains("three") && !debug.contains("text/plain"),
        "{debug}"
    );
}
