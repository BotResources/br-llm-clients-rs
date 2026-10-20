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
fn given_an_https_text_when_built_by_https_then_it_is_accepted() {
    let endpoint = https("https://api.example.com/v1/messages");
    assert!(endpoint.is_https());
    assert_eq!(endpoint.host(), "api.example.com");
    assert_eq!(endpoint.port(), 443);
    assert_eq!(endpoint.path(), "/v1/messages");
}

#[test]
fn given_an_http_text_when_built_by_https_then_the_scheme_is_refused() {
    assert!(refused_https("http://example.com/").contains("https"));
}

#[test]
fn given_an_https_text_when_built_by_plain_http_then_the_scheme_is_refused() {
    let error = Endpoint::plain_http("https://example.com/").unwrap_err();
    assert!(matches!(error, ConfigError::InvalidEndpoint { .. }));
}

#[test]
fn given_an_http_text_when_built_by_plain_http_then_it_is_accepted_and_not_https() {
    let endpoint = Endpoint::plain_http("http://localhost:8080/x").expect("endpoint");
    assert!(!endpoint.is_https());
    assert_eq!(endpoint.port(), 8080);
}

#[test]
fn given_a_scheme_in_capitals_when_built_then_it_is_accepted() {
    assert!(Endpoint::https("HTTPS://Example.com/").is_ok());
}

#[test]
fn given_other_schemes_when_built_by_either_constructor_then_both_refuse() {
    for text in ["ftp://h/", "ws://h/", "file:///etc", "javascript://h/"] {
        assert!(Endpoint::https(text).is_err(), "{text}");
        assert!(Endpoint::plain_http(text).is_err(), "{text}");
    }
}

#[test]
fn given_an_empty_path_when_built_then_the_path_is_a_slash() {
    assert_eq!(https("https://h").path(), "/");
    assert_eq!(https("https://h?a=b").path_and_query(), "/?a=b");
    assert_eq!(https("https://h:8443").path_and_query(), "/");
}

#[test]
fn given_texts_that_are_not_urls_when_built_then_they_are_refused() {
    for text in [
        "",
        "example.com",
        "example.com/x",
        "https:/example.com",
        "https:example.com",
        "://example.com",
        "1http://example.com",
        "ht tp://example.com",
        "https//example.com",
    ] {
        refused_https(text);
    }
}

#[test]
fn given_no_host_when_built_then_it_is_refused() {
    for text in [
        "https:///p",
        "https://:443/",
        "https://?q=1",
        "https://",
        "https://[]/",
    ] {
        refused_https(text);
    }
}

#[test]
fn given_user_information_of_any_kind_when_built_then_it_is_refused() {
    for text in [
        "https://user@h/",
        "https://user:pass@h/",
        "https://:pass@h/",
        "https://@h/",
        "https://u@h:443/p",
    ] {
        assert!(refused_https(text).contains("user information"), "{text}");
    }
}

#[test]
fn given_an_at_sign_after_the_authority_when_built_then_it_is_kept() {
    let endpoint = https("https://h/a@b?x=y@z");
    assert_eq!(endpoint.path_and_query(), "/a@b?x=y@z");
}

#[test]
fn given_a_fragment_when_built_then_it_is_refused() {
    for text in [
        "https://h/#",
        "https://h/p#frag",
        "https://h/p?q=1#f",
        "https://h#f",
    ] {
        assert!(refused_https(text).contains("fragment"), "{text}");
    }
}

#[test]
fn given_a_control_character_when_built_then_it_is_refused() {
    for text in [
        "https://h/\u{1}",
        "https://h/\t",
        "https://h/\n",
        "https://h/\r",
        "https://h/\u{7f}",
        "https://h/?\u{0}",
        "https://\u{1}h/",
    ] {
        assert!(
            refused_https(text).contains("control character"),
            "{text:?}"
        );
    }
}

#[test]
fn given_a_space_when_built_then_it_is_refused() {
    for text in [
        "https://h/a b",
        "https://h/?a=b c",
        "https://h /",
        " https://h/",
        "https://h/ ",
    ] {
        assert!(refused_https(text).contains("space"), "{text:?}");
    }
}

#[test]
fn given_a_backslash_when_built_then_it_is_refused() {
    for text in ["https://h\\p", "https://h/a\\b", "https://h/?q=\\"] {
        assert!(refused_https(text).contains("backslash"), "{text:?}");
    }
}

#[test]
fn given_a_non_ascii_character_when_built_then_it_is_refused() {
    for text in [
        "https://h/é",
        "https://hé/",
        "https://h/?q=ü",
        "https://h/\u{200b}",
        "https://h/\u{85}",
    ] {
        let reason = refused_https(text);
        assert!(
            reason.contains("control character") || reason.contains("non-ASCII"),
            "{text:?}"
        );
    }
    assert!(refused_https("https://h/é").contains("non-ASCII"));
}

#[test]
fn given_a_bad_percent_sign_when_built_then_it_is_refused() {
    for text in [
        "https://h/%",
        "https://h/%4",
        "https://h/%zz",
        "https://h/%g1",
        "https://h/a%",
        "https://h/?q=%",
        "https://h/?q=%1",
        "https://h/?q=%1g",
        "https://h/%%41",
    ] {
        assert!(refused_https(text).contains("percent"), "{text}");
    }
}

#[test]
fn given_good_percent_escapes_when_built_then_the_text_is_kept_as_given() {
    let endpoint = https("https://h/a%2fb%2F%7e?x=%2f%41");
    assert_eq!(endpoint.path_and_query(), "/a%2fb%2F%7e?x=%2f%41");
}

#[test]
fn given_dot_segments_when_built_then_they_are_refused() {
    for text in [
        "https://h/.",
        "https://h/..",
        "https://h/a/./b",
        "https://h/a/../b",
        "https://h/a/..",
        "https://h/a/.",
        "https://h/%2e",
        "https://h/%2E",
        "https://h/%2e%2e",
        "https://h/%2E%2E",
        "https://h/.%2e",
        "https://h/%2e./x",
        "https://h/a/%2e/b",
        "https://h/a/%2E%2e/b",
        "https://h/./",
        "https://h/../",
    ] {
        assert!(refused_https(text).contains("dot segment"), "{text}");
    }
}

#[test]
fn given_segments_that_only_look_like_dots_when_built_then_they_are_accepted() {
    for text in [
        "https://h/.a",
        "https://h/..a",
        "https://h/a.",
        "https://h/a.b",
        "https://h/...",
        "https://h/%2ea",
        "https://h/.../x",
        "https://h/?x=/../",
        "https://h/?.",
    ] {
        https(text);
    }
}

#[test]
fn given_characters_a_url_does_not_allow_when_built_then_they_are_refused() {
    for text in [
        "https://h/a<b",
        "https://h/a\"b",
        "https://h/a{b}",
        "https://h/a|b",
        "https://h/a^b",
        "https://h/a`b",
        "https://h/a[b]",
        "https://h/?q=[1]",
    ] {
        refused_https(text);
    }
}

#[test]
fn given_a_text_longer_than_8192_bytes_when_built_then_it_is_refused() {
    let at_limit = format!("https://h/{}", "a".repeat(8192 - "https://h/".len()));
    assert!(Endpoint::https(&at_limit).is_ok());
    let over = format!("https://h/{}", "a".repeat(8192 - "https://h/".len() + 1));
    assert!(refused_https(&over).contains("8192"));
    let long_query = format!("https://h/?{}", "q".repeat(9000));
    refused_https(&long_query);
}
