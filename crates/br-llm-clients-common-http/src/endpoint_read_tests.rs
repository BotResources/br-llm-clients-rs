use crate::endpoint::Endpoint;

fn https(text: &str) -> Endpoint {
    Endpoint::https(text).unwrap_or_else(|e| panic!("{text:?} refused: {e}"))
}

fn plain(text: &str) -> Endpoint {
    Endpoint::plain_http(text).unwrap_or_else(|e| panic!("{text:?} refused: {e}"))
}

#[test]
fn given_a_host_when_read_then_it_keeps_its_letter_case_and_the_ipv6_brackets() {
    assert_eq!(https("https://Api.Example.COM/x").host(), "Api.Example.COM");
    assert_eq!(https("https://[::1]/x").host(), "[::1]");
    assert_eq!(
        https("https://[2001:db8::1]:8443/x").host(),
        "[2001:db8::1]"
    );
    assert_eq!(plain("http://LocalHost:80/x").host(), "LocalHost");
    assert_eq!(https("https://example.com./x").host(), "example.com.");
}

#[test]
fn given_a_port_in_the_text_when_read_then_it_is_that_port_even_when_it_is_the_default() {
    assert_eq!(https("https://h:8443/").port(), 8443);
    assert_eq!(https("https://h:443/").port(), 443);
    assert_eq!(plain("http://h:443/").port(), 443);
    assert_eq!(plain("http://h:80/").port(), 80);
    assert_eq!(https("https://[::1]:9/").port(), 9);
}

#[test]
fn given_no_port_in_the_text_when_read_then_it_is_the_default_of_the_scheme() {
    assert_eq!(https("https://h/").port(), 443);
    assert_eq!(https("https://h").port(), 443);
    assert_eq!(https("https://h:/x").port(), 443);
    assert_eq!(https("https://[::1]/x").port(), 443);
    assert_eq!(plain("http://h/").port(), 80);
    assert_eq!(plain("http://h:").port(), 80);
}

#[test]
fn given_any_endpoint_when_read_then_the_port_is_never_zero() {
    for endpoint in [
        https("https://h"),
        plain("http://h"),
        https("https://h:1/"),
        https("https://h:65535/"),
        https("https://h/a").join_segment("b").unwrap(),
    ] {
        assert_ne!(endpoint.port(), 0);
    }
}

#[test]
fn given_a_path_when_read_then_it_is_as_given_encoded_and_starts_with_a_slash() {
    assert_eq!(https("https://h/v1/messages").path(), "/v1/messages");
    assert_eq!(https("https://h/a%2Fb/c%20d").path(), "/a%2Fb/c%20d");
    assert_eq!(https("https://h/v1/").path(), "/v1/");
    assert_eq!(https("https://h/v1?q=1").path(), "/v1");
}

#[test]
fn given_an_empty_path_when_read_then_it_is_a_slash() {
    assert_eq!(https("https://h").path(), "/");
    assert_eq!(https("https://h:8443").path(), "/");
    assert_eq!(https("https://h?a=b").path(), "/");
    assert_eq!(https("https://h?").path(), "/");
}

#[test]
fn given_a_joined_segment_when_read_then_the_path_is_the_one_that_will_be_sent() {
    let joined = https("https://h/v1/models")
        .join_segment("gemini:pro/1")
        .unwrap();
    assert_eq!(joined.path(), "/v1/models/gemini:pro%2F1");
    assert_eq!(joined.path_and_query(), joined.path());
}

#[test]
fn given_no_query_when_read_then_there_is_nothing() {
    assert_eq!(https("https://h").query(), None);
    assert_eq!(https("https://h/p").query(), None);
    assert_eq!(
        https("https://h/p").join_segment("q").unwrap().query(),
        None
    );
}

#[test]
fn given_a_query_when_read_then_it_is_as_sent_without_the_question_mark() {
    assert_eq!(
        https("https://h/p?a=b&c=d%20e").query(),
        Some("a=b&c=d%20e")
    );
    assert_eq!(https("https://h?x=1").query(), Some("x=1"));
    assert_eq!(https("https://h/p?a=b?c").query(), Some("a=b?c"));
}

#[test]
fn given_a_text_ending_with_a_question_mark_when_read_then_the_query_is_empty_not_nothing() {
    assert_eq!(https("https://h/p?").query(), Some(""));
    assert_eq!(https("https://h?").query(), Some(""));
    assert_ne!(https("https://h/p?").query(), https("https://h/p").query());
}

#[test]
fn given_query_pairs_when_read_then_they_are_encoded_in_order() {
    let endpoint = https("https://h/p")
        .with_query_pair("api-version", "2024-10-21")
        .unwrap()
        .with_query_pair("a b", "c&d=e")
        .unwrap();
    assert_eq!(
        endpoint.query(),
        Some("api-version=2024-10-21&a%20b=c%26d%3De")
    );
    assert_eq!(
        endpoint.path_and_query(),
        "/p?api-version=2024-10-21&a%20b=c%26d%3De"
    );
}

#[test]
fn given_an_empty_query_when_a_pair_is_added_then_no_leading_ampersand_appears() {
    let endpoint = https("https://h/p?").with_query_pair("a", "b").unwrap();
    assert_eq!(endpoint.query(), Some("a=b"));
}

#[test]
fn given_the_constructor_when_asked_if_https_then_only_the_secure_constructor_says_yes() {
    assert!(https("https://h/").is_https());
    assert!(https("HTTPS://h/").is_https());
    assert!(!plain("http://h/").is_https());
    assert!(!plain("HTTP://h/").is_https());
    assert!(
        https("https://h/")
            .join_segment("a")
            .unwrap()
            .with_query_pair("k", "v")
            .unwrap()
            .is_https()
    );
    assert!(!plain("http://h/").join_segment("a").unwrap().is_https());
}

#[test]
fn given_an_endpoint_when_built_on_then_the_original_reads_the_same() {
    let base = https("https://H:8443/v1?a=b");
    let before = (
        base.host().to_owned(),
        base.port(),
        base.path().to_owned(),
        base.query().map(str::to_owned),
        base.is_https(),
    );
    let _ = base.join_segment("m").unwrap();
    let _ = base.with_query_pair("c", "d").unwrap();
    let after = (
        base.host().to_owned(),
        base.port(),
        base.path().to_owned(),
        base.query().map(str::to_owned),
        base.is_https(),
    );
    assert_eq!(before, after);
    assert_eq!(base.clone().query(), base.query());
}

#[test]
fn given_a_query_when_formatting_then_display_and_debug_still_hide_it() {
    let endpoint = https("https://h:8443/p?secret=1");
    assert_eq!(endpoint.query(), Some("secret=1"));
    assert!(!format!("{endpoint}").contains("secret"));
    assert!(!format!("{endpoint:?}").contains("secret"));
    assert_eq!(format!("{endpoint}"), "https://h:8443/p");
}
