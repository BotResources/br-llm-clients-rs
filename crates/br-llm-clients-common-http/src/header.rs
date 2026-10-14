use http::HeaderName;

const RESERVED_NAMES: [&str; 11] = [
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "keep-alive",
    "proxy-connection",
    "upgrade",
    "te",
    "expect",
    "accept-encoding",
    "user-agent",
];

const MAX_NAME_CHARS: usize = 64;

pub(crate) fn is_reserved(name: &HeaderName) -> bool {
    RESERVED_NAMES.contains(&name.as_str())
}

pub(crate) fn describe_name(name: &str) -> String {
    let mut shown: String = name
        .chars()
        .take(MAX_NAME_CHARS)
        .flat_map(char::escape_debug)
        .collect();
    if name.chars().nth(MAX_NAME_CHARS).is_some() {
        shown.push_str("...");
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_the_ten_reserved_names_when_checking_then_all_are_reserved() {
        for name in RESERVED_NAMES {
            let parsed = HeaderName::from_bytes(name.as_bytes()).expect("valid name");
            assert!(is_reserved(&parsed), "{name}");
        }
    }

    #[test]
    fn given_a_long_name_with_control_characters_when_describing_then_it_is_escaped_and_cut() {
        let described = describe_name(&format!("a\nb{}", "x".repeat(200)));
        assert!(described.starts_with("a\\nb"));
        assert!(described.ends_with("..."));
        assert!(described.len() < 80);
    }
}
