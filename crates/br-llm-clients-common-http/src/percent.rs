const HEX: &[u8; 16] = b"0123456789ABCDEF";

pub(crate) fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

pub(crate) fn is_sub_delim(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
    )
}

pub(crate) fn is_pchar(byte: u8) -> bool {
    is_unreserved(byte) || is_sub_delim(byte) || matches!(byte, b':' | b'@')
}

pub(crate) fn encode_segment(text: &str) -> String {
    encode(text, is_pchar)
}

pub(crate) fn encode_query_component(text: &str) -> String {
    encode(text, is_unreserved)
}

fn encode(text: &str, keep: fn(u8) -> bool) -> String {
    let mut out = String::with_capacity(text.len());
    for &byte in text.as_bytes() {
        if keep(byte) {
            out.push(char::from(byte));
        } else {
            push_escaped(&mut out, byte);
        }
    }
    out
}

fn push_escaped(out: &mut String, byte: u8) {
    out.push('%');
    out.push(char::from(nibble_char(byte >> 4)));
    out.push(char::from(nibble_char(byte & 0x0f)));
}

fn nibble_char(nibble: u8) -> u8 {
    HEX.get(usize::from(nibble)).copied().unwrap_or(b'0')
}

pub(crate) fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0'),
        b'a'..=b'f' => byte.checked_sub(b'a').and_then(|v| v.checked_add(10)),
        b'A'..=b'F' => byte.checked_sub(b'A').and_then(|v| v.checked_add(10)),
        _ => None,
    }
}

pub(crate) fn decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while let Some(&byte) = bytes.get(index) {
        let escaped = if byte == b'%' {
            decode_escape(bytes, index)
        } else {
            None
        };
        match escaped {
            Some(decoded) => {
                out.push(decoded);
                index = index.saturating_add(3);
            }
            None => {
                out.push(byte);
                index = index.saturating_add(1);
            }
        }
    }
    out
}

fn decode_escape(bytes: &[u8], index: usize) -> Option<u8> {
    let high = hex_value(*bytes.get(index.checked_add(1)?)?)?;
    let low = hex_value(*bytes.get(index.checked_add(2)?)?)?;
    high.checked_mul(16)?.checked_add(low)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_slash_when_encoding_a_segment_then_it_is_escaped_and_a_colon_is_kept() {
        assert_eq!(encode_segment("a/b:c"), "a%2Fb:c");
    }

    #[test]
    fn given_a_percent_sign_when_encoding_a_segment_then_it_is_escaped() {
        assert_eq!(encode_segment("100%"), "100%25");
    }

    #[test]
    fn given_non_ascii_when_encoding_a_segment_then_every_utf8_byte_is_escaped() {
        assert_eq!(encode_segment("é"), "%C3%A9");
    }

    #[test]
    fn given_sub_delims_and_at_when_encoding_a_segment_then_they_are_kept() {
        assert_eq!(encode_segment("a!$&'()*+,;=@b"), "a!$&'()*+,;=@b");
    }

    #[test]
    fn given_reserved_characters_when_encoding_a_query_component_then_only_unreserved_are_kept() {
        assert_eq!(
            encode_query_component("a b&c=d/e:f~g"),
            "a%20b%26c%3Dd%2Fe%3Af~g"
        );
    }

    #[test]
    fn given_mixed_case_escapes_when_decoding_then_bytes_come_back() {
        assert_eq!(decode("%2e%2E%41"), b"..A");
    }

    #[test]
    fn given_a_lone_percent_when_decoding_then_it_is_kept() {
        assert_eq!(decode("%2"), b"%2");
    }
}
