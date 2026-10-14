use http::{HeaderMap, HeaderValue};

use crate::body_error::BodyError;
use crate::encoding::unsupported;

fn headers(values: &[&str]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for value in values {
        map.append(
            "content-encoding",
            HeaderValue::from_str(value).expect("value"),
        );
    }
    map
}

fn encoding_of(values: &[&str]) -> Option<String> {
    match unsupported(&headers(values)) {
        Some(BodyError::UnsupportedEncoding { encoding, .. }) => Some(encoding),
        Some(other) => panic!("unexpected {other:?}"),
        None => None,
    }
}

#[test]
fn given_no_header_or_identity_in_any_case_when_checking_then_it_is_supported() {
    assert_eq!(unsupported(&HeaderMap::new()), None);
    for value in [
        "identity",
        "Identity",
        "IDENTITY",
        " identity ",
        "identity, identity",
        "",
    ] {
        assert_eq!(encoding_of(&[value]), None, "{value:?}");
    }
}

#[test]
fn given_another_coding_when_checking_then_it_is_named() {
    assert_eq!(encoding_of(&["gzip"]).as_deref(), Some("gzip"));
    assert_eq!(encoding_of(&["GZIP"]).as_deref(), Some("GZIP"));
    assert_eq!(encoding_of(&["br"]).as_deref(), Some("br"));
    assert_eq!(encoding_of(&["zstd"]).as_deref(), Some("zstd"));
}

#[test]
fn given_a_list_with_identity_first_when_checking_then_the_other_coding_is_named() {
    assert_eq!(encoding_of(&["identity, gzip"]).as_deref(), Some("gzip"));
    assert_eq!(
        encoding_of(&["identity", "deflate"]).as_deref(),
        Some("deflate")
    );
}

#[test]
fn given_a_value_that_is_not_a_registered_coding_when_checking_then_it_is_not_echoed() {
    let long = "a".repeat(200);
    assert_eq!(encoding_of(&[&long]).as_deref(), Some("(unrecognised)"));
    assert_eq!(encoding_of(&["x y"]).as_deref(), Some("(unrecognised)"));
    assert_eq!(
        encoding_of(&["x=\"sk-1\""]).as_deref(),
        Some("(unrecognised)")
    );
}

#[test]
fn given_a_value_that_is_not_text_when_checking_then_it_is_unsupported() {
    let mut map = HeaderMap::new();
    map.append(
        "content-encoding",
        HeaderValue::from_bytes(&[0xff, 0xfe]).expect("value"),
    );
    assert!(matches!(
        unsupported(&map),
        Some(BodyError::UnsupportedEncoding { .. })
    ));
}
