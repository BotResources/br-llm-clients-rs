use crate::config_error::ConfigError;
use crate::endpoint::Endpoint;
use crate::request::Request;

fn endpoint() -> Endpoint {
    Endpoint::https("https://h/v1?key=QUERY-SECRET").expect("endpoint")
}

fn request() -> Request {
    Request::new(endpoint(), "BODY-SECRET")
}

#[test]
fn given_valid_headers_when_added_then_their_names_are_kept_in_order_in_lower_case() {
    let built = request()
        .with_header("Content-Type", "application/json")
        .and_then(|r| r.with_header("X-Trace", "abc"))
        .expect("headers");
    assert_eq!(
        built.header_names().collect::<Vec<_>>(),
        ["content-type", "x-trace"]
    );
    assert_eq!(built.body_len(), 11);
}

#[test]
fn given_an_invalid_name_when_added_then_it_is_an_invalid_header() {
    for name in ["", "bad name", "a:b", "é", "a\nb", "(x)", "a/b"] {
        let error = request().with_header(name, "v").unwrap_err();
        assert!(
            matches!(error, ConfigError::InvalidHeader { .. }),
            "{name:?}: {error:?}"
        );
    }
}

#[test]
fn given_an_invalid_value_when_added_then_it_is_an_invalid_header_and_the_text_hides_the_value() {
    for value in ["a\nb", "tok\n", "tok\r\n", "a\u{0}b", "a\u{7f}"] {
        let error = request().with_header("x-key", value).unwrap_err();
        assert!(
            matches!(error, ConfigError::InvalidHeader { .. }),
            "{value:?}"
        );
        assert!(error.to_string().contains("x-key"));
        assert!(!error.to_string().contains("tok"), "{error}");
    }
}

#[test]
fn given_a_value_with_tab_and_inner_spaces_when_added_then_it_is_accepted() {
    assert!(request().with_header("x-a", "a b\tc").is_ok());
}

#[test]
fn given_a_reserved_name_in_any_case_when_added_then_it_is_a_reserved_header() {
    for name in [
        "Host",
        "content-length",
        "TRANSFER-ENCODING",
        "Connection",
        "Keep-Alive",
        "Proxy-Connection",
        "Upgrade",
        "TE",
        "expect",
        "Accept-Encoding",
    ] {
        let error = request().with_header(name, "x").unwrap_err();
        assert!(
            matches!(error, ConfigError::ReservedHeader { .. }),
            "{name}"
        );
    }
}

#[test]
fn given_names_the_importer_may_set_when_added_then_they_are_accepted() {
    for name in [
        "content-type",
        "accept",
        "user-agent",
        "authorization",
        "x-anything",
        "content-encoding",
        "origin",
    ] {
        assert!(request().with_header(name, "x").is_ok(), "{name}");
    }
}

#[test]
fn given_the_same_name_twice_in_any_case_when_added_then_it_is_a_duplicate_header() {
    let error = request()
        .with_header("X-Key", "1")
        .and_then(|r| r.with_header("x-key", "2"))
        .unwrap_err();
    assert!(matches!(error, ConfigError::DuplicateHeader { .. }));
}

#[test]
fn given_a_request_when_formatting_debug_then_only_names_and_body_size_show() {
    let built = request()
        .with_header("x-api-key", "HEADER-SECRET")
        .expect("header");
    let debug = format!("{built:?}");
    assert!(debug.contains("x-api-key"));
    assert!(debug.contains("body_len: 11"));
    for leaked in ["HEADER-SECRET", "BODY-SECRET", "QUERY-SECRET", "key="] {
        assert!(!debug.contains(leaked), "{debug}");
    }
}
