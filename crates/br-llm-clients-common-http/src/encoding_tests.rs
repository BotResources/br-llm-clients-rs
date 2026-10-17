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

fn is_unsupported(values: &[&str]) -> bool {
    matches!(
        unsupported(&headers(values)),
        Some(BodyError::UnsupportedEncoding { .. })
    )
}

#[test]
fn given_no_header_or_identity_in_any_case_when_checking_then_it_is_supported() {
    assert!(unsupported(&HeaderMap::new()).is_none());
    for value in [
        "identity",
        "Identity",
        "IDENTITY",
        " identity ",
        "identity, identity",
        "",
    ] {
        assert!(!is_unsupported(&[value]), "{value:?}");
    }
}

#[test]
fn given_another_coding_when_checking_then_it_is_unsupported() {
    for value in ["gzip", "GZIP", "br", "zstd", "x-unknown", "identity, gzip"] {
        assert!(is_unsupported(&[value]), "{value}");
    }
    assert!(is_unsupported(&["identity", "deflate"]));
}

#[test]
fn given_a_value_that_is_not_text_when_checking_then_it_is_unsupported() {
    let mut map = HeaderMap::new();
    map.append(
        "content-encoding",
        HeaderValue::from_bytes(&[0xff, 0xfe]).expect("value"),
    );
    assert!(unsupported(&map).is_some());
}

#[test]
fn given_an_odd_value_when_checking_then_the_error_text_never_holds_it() {
    let error = unsupported(&headers(&["sk-token-1, SECRET-CODING"])).expect("error");
    let text = format!("{error} {error:?}");
    assert!(
        !text.contains("sk-token-1") && !text.contains("SECRET-CODING"),
        "{text}"
    );
}
