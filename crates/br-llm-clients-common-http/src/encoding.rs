use http::HeaderMap;
use http::header::CONTENT_ENCODING;

use crate::body_error::BodyError;

pub(crate) fn unsupported(headers: &HeaderMap) -> Option<BodyError> {
    let other_than_identity = headers.get_all(CONTENT_ENCODING).iter().any(|value| {
        let Ok(text) = value.to_str() else {
            return true;
        };
        text.split(',')
            .map(str::trim)
            .filter(|coding| !coding.is_empty())
            .any(|coding| !coding.eq_ignore_ascii_case("identity"))
    });
    other_than_identity.then(|| BodyError::UnsupportedEncoding {
        text: "the response has a Content-Encoding other than identity and this library does not decode it; read the header from the response".to_owned(),
    })
}
