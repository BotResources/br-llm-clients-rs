use crate::config_error::ConfigError;
use crate::endpoint::Endpoint;

fn https(text: &str) -> Endpoint {
    Endpoint::https(text).unwrap_or_else(|e| panic!("{text:?} refused: {e}"))
}

fn refused_https(text: &str) -> String {
    match Endpoint::https(text) {
        Err(ConfigError::InvalidEndpoint { text }) => text,
        other => panic!("{text:?} should be InvalidEndpoint, got {other:?}"),
    }
}

#[test]
fn given_a_segment_when_joining_then_a_slash_is_encoded_and_a_colon_is_kept() {
    let base = https("https://h/v1/models");
    assert_eq!(base.join_segment("a/b").unwrap().path(), "/v1/models/a%2Fb");
    assert_eq!(
        base.join_segment("gemini:generate").unwrap().path(),
        "/v1/models/gemini:generate"
    );
    assert_eq!(
        base.join_segment("100%").unwrap().path(),
        "/v1/models/100%25"
    );
    assert_eq!(base.join_segment("é").unwrap().path(), "/v1/models/%C3%A9");
    assert_eq!(base.join_segment("a b").unwrap().path(), "/v1/models/a%20b");
    assert_eq!(
        base.join_segment("a?b#c").unwrap().path(),
        "/v1/models/a%3Fb%23c"
    );
    assert_eq!(
        base.join_segment("a\\b").unwrap().path(),
        "/v1/models/a%5Cb"
    );
    assert_eq!(
        base.join_segment("a\nb").unwrap().path(),
        "/v1/models/a%0Ab"
    );
    assert_eq!(
        base.join_segment("a!$&'()*+,;=@~_-.b").unwrap().path(),
        "/v1/models/a!$&'()*+,;=@~_-.b"
    );
}

#[test]
fn given_a_segment_that_looks_encoded_when_joining_then_the_percent_is_encoded_so_it_stays_literal()
{
    let joined = https("https://h/").join_segment("%2e%2e").unwrap();
    assert_eq!(joined.path(), "/%252e%252e");
}

#[test]
fn given_an_empty_dot_or_dotdot_segment_when_joining_then_it_is_refused() {
    let base = https("https://h/v1");
    for segment in ["", ".", ".."] {
        assert!(
            matches!(
                base.join_segment(segment),
                Err(ConfigError::InvalidEndpoint { .. })
            ),
            "{segment:?}"
        );
    }
    assert!(base.join_segment("...").is_ok());
    assert!(base.join_segment(".a").is_ok());
}

#[test]
fn given_roots_and_trailing_slashes_when_joining_then_no_double_slash_appears() {
    assert_eq!(https("https://h").join_segment("m").unwrap().path(), "/m");
    assert_eq!(https("https://h/").join_segment("m").unwrap().path(), "/m");
    assert_eq!(
        https("https://h/v1/").join_segment("m").unwrap().path(),
        "/v1/m"
    );
    assert_eq!(
        https("https://h/v1").join_segment("m").unwrap().path(),
        "/v1/m"
    );
}

#[test]
fn given_a_query_when_joining_then_the_query_is_kept_and_the_original_is_unchanged() {
    let base = https("https://h/v1?k=v");
    let joined = base.join_segment("m").unwrap();
    assert_eq!(joined.path_and_query(), "/v1/m?k=v");
    assert_eq!(base.path_and_query(), "/v1?k=v");
}

#[test]
fn given_a_pair_when_adding_a_query_pair_then_name_and_value_are_percent_encoded() {
    let base = https("https://h/p");
    let with = base.with_query_pair("a b", "c&d=e/é").expect("pair");
    assert_eq!(with.path_and_query(), "/p?a%20b=c%26d%3De%2F%C3%A9");
    assert_eq!(base.path_and_query(), "/p");
}

#[test]
fn given_a_query_when_adding_pairs_then_they_are_appended_with_an_ampersand() {
    let endpoint = https("https://h/p?x=1")
        .with_query_pair("y", "2")
        .and_then(|e| e.with_query_pair("z", ""))
        .expect("pairs");
    assert_eq!(endpoint.path_and_query(), "/p?x=1&y=2&z=");
    assert_eq!(
        https("https://h/p?")
            .with_query_pair("y", "2")
            .expect("pair")
            .path_and_query(),
        "/p?y=2"
    );
}

#[test]
fn given_a_query_when_formatting_then_neither_debug_nor_display_shows_it() {
    let endpoint = https("https://h:8443/v1/m?key=SECRET-KEY&other=1")
        .with_query_pair("tok", "SECRET-TOKEN")
        .expect("pair");
    let debug = format!("{endpoint:?}");
    let display = endpoint.to_string();
    for text in [&debug, &display] {
        assert!(!text.contains("SECRET"), "{text}");
        assert!(!text.contains("key="), "{text}");
        assert!(!text.contains('?'), "{text}");
    }
    assert!(
        debug.contains("https")
            && debug.contains("\"h\"")
            && debug.contains("8443")
            && debug.contains("/v1/m")
    );
    assert_eq!(display, "https://h:8443/v1/m");
}

#[test]
fn given_an_invalid_text_when_refused_then_the_error_text_never_echoes_the_input() {
    for text in [
        "https://user:PASSWORD@h/?token=SECRET",
        "https://h/p?token=SECRET#frag",
        "https://h/p?token=SECRET\u{1}",
        "https://h/p?token=SECRET%zz",
    ] {
        let reason = refused_https(text);
        assert!(
            !reason.contains("SECRET") && !reason.contains("PASSWORD"),
            "{reason}"
        );
    }
}

#[test]
fn given_a_result_longer_than_8192_bytes_when_joining_or_adding_a_pair_then_it_is_refused() {
    let base = https("https://h/p");
    assert!(matches!(
        base.join_segment(&"y".repeat(9000)),
        Err(ConfigError::InvalidEndpoint { .. })
    ));
    assert!(matches!(
        base.with_query_pair("a", &"x".repeat(9000)),
        Err(ConfigError::InvalidEndpoint { .. })
    ));
    let near = base.join_segment(&"y".repeat(8100)).expect("fits");
    assert!(near.with_query_pair("a", &"x".repeat(200)).is_err());
    assert!(near.with_query_pair("a", "x").is_ok());
}
