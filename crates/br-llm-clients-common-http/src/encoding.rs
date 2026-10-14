use http::HeaderMap;
use http::header::CONTENT_ENCODING;

use crate::body_error::BodyError;

const KNOWN_CODINGS: [&str; 12] = [
    "gzip",
    "x-gzip",
    "deflate",
    "compress",
    "x-compress",
    "br",
    "zstd",
    "dcb",
    "dcz",
    "exi",
    "pack200-gzip",
    "aes128gcm",
];

const UNRECOGNISED: &str = "(unrecognised)";

pub(crate) fn unsupported(headers: &HeaderMap) -> Option<BodyError> {
    for value in headers.get_all(CONTENT_ENCODING) {
        let text = value.to_str().unwrap_or("");
        if value.to_str().is_err() {
            return Some(error(UNRECOGNISED));
        }
        for coding in text.split(',').map(str::trim).filter(|c| !c.is_empty()) {
            if !coding.eq_ignore_ascii_case("identity") {
                return Some(error(coding));
            }
        }
    }
    None
}

fn error(coding: &str) -> BodyError {
    let known = KNOWN_CODINGS
        .iter()
        .any(|name| name.eq_ignore_ascii_case(coding));
    let encoding = if known {
        coding.to_owned()
    } else {
        UNRECOGNISED.to_owned()
    };
    let text = format!(
        "the response body is encoded with `{encoding}` and this library does not decode it"
    );
    BodyError::UnsupportedEncoding { encoding, text }
}
