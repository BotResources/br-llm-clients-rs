use std::net::Ipv6Addr;

use crate::config_error::ConfigError;
use crate::endpoint_scheme::Scheme;
use crate::percent::{decode, hex_value, is_pchar};

pub(crate) struct Parsed {
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
    pub(crate) path: String,
    pub(crate) query: Option<String>,
}

pub(crate) fn parse(text: &str, expected: Scheme) -> Result<Parsed, ConfigError> {
    screen(text)?;
    let rest = strip_scheme(text, expected)?;
    let (authority, tail) = split_at_first(rest, &['/', '?']);
    let (host, port) = parse_authority(authority)?;
    let (path, query) = match tail.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (tail, None),
    };
    let path = if path.is_empty() { "/" } else { path };
    check_component(path, "path", false)?;
    check_dot_segments(path)?;
    if let Some(query) = query {
        check_component(query, "query", true)?;
    }
    Ok(Parsed {
        host,
        port,
        path: path.to_owned(),
        query: query.map(str::to_owned),
    })
}

fn screen(text: &str) -> Result<(), ConfigError> {
    for c in text.chars() {
        let reason = if c.is_ascii_control() {
            "it holds a control character"
        } else if c == ' ' {
            "it holds a space"
        } else if c == '\\' {
            "it holds a backslash"
        } else if !c.is_ascii() {
            "it holds a non-ASCII character"
        } else if c == '#' {
            "it holds a fragment"
        } else {
            continue;
        };
        return Err(ConfigError::endpoint(reason));
    }
    Ok(())
}

fn strip_scheme(text: &str, expected: Scheme) -> Result<&str, ConfigError> {
    let not_a_url = || ConfigError::endpoint("it is not a valid URL");
    let (scheme, rest) = text.split_once(':').ok_or_else(not_a_url)?;
    let rest = rest.strip_prefix("//").ok_or_else(not_a_url)?;
    let mut chars = scheme.chars();
    let well_formed = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !well_formed {
        return Err(not_a_url());
    }
    if !scheme.eq_ignore_ascii_case(expected.as_str()) {
        return Err(ConfigError::endpoint(format!(
            "the scheme must be `{}`",
            expected.as_str()
        )));
    }
    Ok(rest)
}

fn split_at_first<'a>(text: &'a str, separators: &[char]) -> (&'a str, &'a str) {
    match text.find(separators) {
        Some(index) => text.split_at(index),
        None => (text, ""),
    }
}

fn parse_authority(authority: &str) -> Result<(String, Option<u16>), ConfigError> {
    if authority.is_empty() {
        return Err(ConfigError::endpoint("it has no host"));
    }
    if authority.contains('@') {
        return Err(ConfigError::endpoint("it holds user information"));
    }
    let (host, port_text) = if authority.starts_with('[') {
        split_ipv6(authority)?
    } else {
        split_name(authority)?
    };
    if host.is_empty() {
        return Err(ConfigError::endpoint("it has no host"));
    }
    Ok((host.to_owned(), parse_port(port_text)?))
}

fn split_ipv6(authority: &str) -> Result<(&str, Option<&str>), ConfigError> {
    let invalid = || ConfigError::endpoint("the host is not a valid IPv6 address");
    let close = authority.find(']').ok_or_else(invalid)?;
    let host = authority.get(..=close).ok_or_else(invalid)?;
    let inner = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .ok_or_else(invalid)?;
    inner.parse::<Ipv6Addr>().map_err(|_| invalid())?;
    let after = authority
        .get(close.saturating_add(1)..)
        .ok_or_else(invalid)?;
    match after {
        "" => Ok((host, None)),
        _ => after
            .strip_prefix(':')
            .map(|port| (host, Some(port)))
            .ok_or_else(|| ConfigError::endpoint("the port is not valid")),
    }
}

fn split_name(authority: &str) -> Result<(&str, Option<&str>), ConfigError> {
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    check_host_name(host)?;
    Ok((host, port))
}

fn check_host_name(host: &str) -> Result<(), ConfigError> {
    let invalid = || ConfigError::endpoint("the host is not a valid host name");
    if host.is_empty() {
        return Err(ConfigError::endpoint("it has no host"));
    }
    if host.len() > 253 {
        return Err(invalid());
    }
    let trimmed = host.strip_suffix('.').unwrap_or(host);
    let labels_valid = trimmed.split('.').all(|label| {
        !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    });
    if labels_valid { Ok(()) } else { Err(invalid()) }
}

fn parse_port(port: Option<&str>) -> Result<Option<u16>, ConfigError> {
    let invalid = || ConfigError::endpoint("the port is not valid");
    match port {
        None | Some("") => Ok(None),
        Some(digits) => {
            if !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(invalid());
            }
            match digits.parse::<u16>() {
                Ok(0) | Err(_) => Err(invalid()),
                Ok(port) => Ok(Some(port)),
            }
        }
    }
}

fn check_component(text: &str, label: &str, allow_question_mark: bool) -> Result<(), ConfigError> {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%' {
            let digits = (
                bytes
                    .get(index.saturating_add(1))
                    .copied()
                    .and_then(hex_value),
                bytes
                    .get(index.saturating_add(2))
                    .copied()
                    .and_then(hex_value),
            );
            if digits.0.is_none() || digits.1.is_none() {
                return Err(ConfigError::endpoint(
                    "it holds a percent sign that is not followed by two hexadecimal digits",
                ));
            }
            index = index.saturating_add(3);
            continue;
        }
        let allowed = byte == b'/' || (allow_question_mark && byte == b'?') || is_pchar(byte);
        if !allowed {
            return Err(ConfigError::endpoint(format!(
                "the {label} holds a character that a URL does not allow there"
            )));
        }
        index = index.saturating_add(1);
    }
    Ok(())
}

fn check_dot_segments(path: &str) -> Result<(), ConfigError> {
    for segment in path.split('/') {
        let decoded = decode(segment);
        if decoded == b"." || decoded == b".." {
            return Err(ConfigError::endpoint("the path holds a dot segment"));
        }
    }
    Ok(())
}
