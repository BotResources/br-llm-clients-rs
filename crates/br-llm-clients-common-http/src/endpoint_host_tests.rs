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
fn given_ports_when_built_then_only_a_number_in_range_is_accepted() {
    assert_eq!(https("https://h:8443/").port(), Some(8443));
    assert_eq!(https("https://h:/").port(), None);
    for text in [
        "https://h:0/",
        "https://h:65536/",
        "https://h:abc/",
        "https://h:80:80/",
        "https://h:-1/",
        "https://h:+1/",
        "https://h:99999999999/",
    ] {
        refused_https(text);
    }
}

#[test]
fn given_host_names_when_built_then_bad_ones_are_refused() {
    for text in [
        "https://a..b/",
        "https://.a/",
        "https://a%41/",
        "https://a b/",
        "https://a_b.c-d.example/x",
    ] {
        let accepted = Endpoint::https(text).is_ok();
        assert_eq!(accepted, text.contains("a_b"), "{text}");
    }
    assert!(Endpoint::https("https://example.com./").is_ok());
}

#[test]
fn given_ipv6_literals_when_built_then_valid_ones_are_accepted_with_brackets() {
    let endpoint = https("https://[::1]:8443/p");
    assert_eq!(endpoint.host(), "[::1]");
    assert_eq!(endpoint.dns_name(), "::1");
    assert_eq!(endpoint.host_header(), "[::1]:8443");
    for text in [
        "https://[::1/",
        "https://[zz]/",
        "https://[::1]x/",
        "https://[::1]:x/",
        "https://[fe80::1%25eth0]/",
    ] {
        refused_https(text);
    }
}

#[test]
fn given_loopback_and_private_addresses_when_built_then_they_are_not_special() {
    for text in [
        "https://127.0.0.1/",
        "https://10.0.0.1:1/",
        "https://192.168.1.1/",
        "https://[::1]/",
        "https://localhost/",
        "https://169.254.169.254/",
    ] {
        https(text);
    }
}

#[test]
fn given_default_and_other_ports_when_asking_the_host_header_then_only_other_ports_show() {
    assert_eq!(https("https://h/").host_header(), "h");
    assert_eq!(https("https://h:443/").host_header(), "h");
    assert_eq!(https("https://h:8443/").host_header(), "h:8443");
    assert_eq!(https("https://h:80/").host_header(), "h:80");
    let plain = |t: &str| Endpoint::plain_http(t).expect("endpoint");
    assert_eq!(plain("http://h/").host_header(), "h");
    assert_eq!(plain("http://h:80/").host_header(), "h");
    assert_eq!(plain("http://h:8080/").host_header(), "h:8080");
    assert_eq!(plain("http://h:443/").host_header(), "h:443");
}

#[test]
fn given_a_text_when_built_then_the_path_and_query_sent_are_the_ones_given() {
    for (text, expected) in [
        ("https://h/a//b/", "/a//b/"),
        ("https://h/a?b=c&d", "/a?b=c&d"),
        ("https://h/p?", "/p?"),
        ("https://h/?a", "/?a"),
        ("https://h//x", "//x"),
        ("https://h/a%2Fb?c=%3D", "/a%2Fb?c=%3D"),
        ("https://H.Example/A?Q=1", "/A?Q=1"),
    ] {
        assert_eq!(https(text).path_and_query(), expected, "{text}");
    }
}
