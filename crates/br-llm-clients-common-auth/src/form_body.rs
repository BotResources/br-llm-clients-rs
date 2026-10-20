use crate::token_form::{FieldValue, TokenForm};

pub(crate) fn encode(form: &TokenForm) -> String {
    let mut body = String::new();
    for (index, (name, value)) in form.fields.iter().enumerate() {
        if index > 0 {
            body.push('&');
        }
        percent_encode(name, &mut body);
        body.push('=');
        match value {
            FieldValue::Plain(text) => percent_encode(text, &mut body),
            FieldValue::Secret(secret) => percent_encode(secret.expose(), &mut body),
        }
    }
    body
}

pub(crate) fn percent_encode(text: &str, out: &mut String) {
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(upper_hex(byte >> 4));
            out.push(upper_hex(byte & 0x0F));
        }
    }
}

fn upper_hex(nibble: u8) -> char {
    char::from_digit(u32::from(nibble), 16)
        .map(|digit| digit.to_ascii_uppercase())
        .unwrap_or('0')
}
