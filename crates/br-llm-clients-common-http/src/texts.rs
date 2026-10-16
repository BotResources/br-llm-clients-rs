use std::error::Error;
use std::io;

use rustls::{CertificateError, Error as TlsError};

pub(crate) const MAX_TEXT_CHARS: usize = 400;

pub(crate) fn bounded(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' {
                c
            } else {
                '?'
            }
        })
        .take(MAX_TEXT_CHARS)
        .collect()
}

pub(crate) fn dns_failure(error: &io::Error) -> String {
    format!("name resolution failed: {}", error.kind())
}

pub(crate) fn tcp_failure(error: &io::Error) -> String {
    format!("could not connect: {}", error.kind())
}

pub(crate) fn tls_failure(error: &io::Error) -> String {
    match error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<TlsError>())
    {
        Some(TlsError::InvalidCertificate(reason)) => certificate_text(reason),
        Some(TlsError::AlertReceived(_)) => {
            "the server ended the handshake with a TLS alert".to_owned()
        }
        Some(_) => "the TLS handshake failed".to_owned(),
        None => format!("the TLS handshake failed: {}", error.kind()),
    }
}

fn certificate_text(reason: &CertificateError) -> String {
    let what = match reason {
        CertificateError::Expired | CertificateError::ExpiredContext { .. } => "has expired",
        CertificateError::NotValidYet | CertificateError::NotValidYetContext { .. } => {
            "is not valid yet"
        }
        CertificateError::UnknownIssuer => "has an unknown issuer",
        CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. } => {
            "is not valid for this host"
        }
        CertificateError::BadSignature => "has a bad signature",
        CertificateError::Revoked => "is revoked",
        _ => "was refused",
    };
    format!("the server certificate {what}")
}

pub(crate) fn find_source<'a, T: Error + 'static>(
    error: &'a (dyn Error + 'static),
) -> Option<&'a T> {
    let mut current: Option<&(dyn Error + 'static)> = Some(error);
    while let Some(layer) = current {
        if let Some(found) = layer.downcast_ref::<T>() {
            return Some(found);
        }
        current = layer.source();
    }
    None
}

pub(crate) fn hyper_text(error: &hyper::Error) -> String {
    if let Some(h2_error) = find_source::<h2::Error>(error) {
        return h2_text(h2_error);
    }
    if let Some(io_error) = find_source::<io::Error>(error) {
        return format!("the connection failed: {}", io_error.kind());
    }
    let text = if error.is_parse() || error.is_parse_version_h2() {
        "the response head is not valid HTTP or is too large"
    } else if error.is_incomplete_message() {
        "the connection closed before the message was complete"
    } else if error.is_canceled() || error.is_closed() {
        "the connection was closed"
    } else if error.is_user() {
        "the connection could not carry the request"
    } else {
        "the connection failed"
    };
    text.to_owned()
}

fn h2_text(error: &h2::Error) -> String {
    let kind = if error.is_library() && error.is_go_away() {
        "the HTTP/2 connection was closed after a broken rule"
    } else if error.is_go_away() {
        "the server is closing the connection"
    } else if error.is_reset() {
        "the stream was reset"
    } else if let Some(io_error) = error.get_io() {
        return format!("the HTTP/2 connection failed: {}", io_error.kind());
    } else {
        "the HTTP/2 exchange failed"
    };
    match error.reason() {
        Some(reason) => format!("{kind}, HTTP/2 reason: {}", reason.description()),
        None => kind.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_long_text_when_bounding_then_it_is_cut_at_the_limit() {
        assert_eq!(bounded(&"a".repeat(2000)).chars().count(), MAX_TEXT_CHARS);
        assert_eq!(bounded("a\"b\\c\nd\u{e9}"), "a?b?c?d?");
        assert_eq!(bounded("short"), "short");
    }

    #[test]
    fn given_an_io_error_with_text_when_describing_then_only_its_kind_shows() {
        let error = io::Error::new(io::ErrorKind::ConnectionRefused, "SECRET-DETAIL 10.0.0.1");
        for text in [
            dns_failure(&error),
            tcp_failure(&error),
            tls_failure(&error),
        ] {
            assert!(!text.contains("SECRET-DETAIL"), "{text}");
        }
    }

    #[test]
    fn given_a_tls_error_wrapped_in_io_when_describing_then_fixed_wording_is_used() {
        let inner = TlsError::InvalidCertificate(CertificateError::UnknownIssuer);
        let error = io::Error::other(inner);
        assert_eq!(
            tls_failure(&error),
            "the server certificate has an unknown issuer"
        );
        let other = io::Error::other(TlsError::General("SECRET-DETAIL".to_owned()));
        assert_eq!(tls_failure(&other), "the TLS handshake failed");
    }

    #[test]
    fn given_an_io_error_when_describing_a_dial_then_the_text_is_the_reason_a_colon_and_the_kind() {
        let error = io::Error::from(io::ErrorKind::ConnectionRefused);
        assert_eq!(
            dns_failure(&error),
            "name resolution failed: connection refused"
        );
        assert_eq!(tcp_failure(&error), "could not connect: connection refused");
        assert_eq!(
            tls_failure(&io::Error::from(io::ErrorKind::UnexpectedEof)),
            "the TLS handshake failed: unexpected end of file"
        );
    }

    #[test]
    fn given_each_certificate_fault_when_describing_then_the_text_has_the_fixed_wording_and_no_kind()
     {
        let cases = [
            (CertificateError::Expired, "has expired"),
            (CertificateError::NotValidYet, "is not valid yet"),
            (CertificateError::UnknownIssuer, "has an unknown issuer"),
            (
                CertificateError::NotValidForName,
                "is not valid for this host",
            ),
            (CertificateError::BadSignature, "has a bad signature"),
            (CertificateError::Revoked, "is revoked"),
            (CertificateError::BadEncoding, "was refused"),
        ];
        for (fault, what) in cases {
            let error = io::Error::other(TlsError::InvalidCertificate(fault));
            assert_eq!(
                tls_failure(&error),
                format!("the server certificate {what}")
            );
        }
    }

    #[test]
    fn given_a_tls_alert_when_describing_then_the_text_names_no_alert() {
        let alert = TlsError::AlertReceived(rustls::AlertDescription::HandshakeFailure);
        assert_eq!(
            tls_failure(&io::Error::other(alert)),
            "the server ended the handshake with a TLS alert"
        );
    }

    #[test]
    fn given_an_http2_error_when_describing_then_the_text_is_the_fixed_wording_a_comma_and_the_reason()
     {
        let failed = h2::Error::from(h2::Reason::CANCEL);
        assert_eq!(
            h2_text(&failed),
            "the HTTP/2 exchange failed, HTTP/2 reason: stream no longer needed"
        );
    }
}
