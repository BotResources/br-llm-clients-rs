use br_llm_clients_common_http::{AuthorizerError, BodyError, SendError};
use br_llm_clients_common_sse::DecodeError;
use br_llm_clients_core::ClientError;

use crate::failure_map::{body_failure, decode_failure, send_failure};

fn text() -> String {
    "t".to_owned()
}

#[test]
fn given_a_failure_before_the_headers_when_mapped_then_it_follows_the_generic_table() {
    let rows = [
        (
            SendError::Authorizer(AuthorizerError::unavailable("u")),
            ClientError::Authentication,
        ),
        (
            SendError::Authorizer(AuthorizerError::fetch_failed("f")),
            ClientError::Transport,
        ),
        (
            SendError::InvalidAuthorization { text: text() },
            ClientError::Authentication,
        ),
        (SendError::Connect { text: text() }, ClientError::Transport),
        (SendError::Tls { text: text() }, ClientError::Transport),
        (
            SendError::ConnectTimeout { text: text() },
            ClientError::Transport,
        ),
        (
            SendError::HeadersTimeout { text: text() },
            ClientError::Transport,
        ),
        (
            SendError::TotalTimeout { text: text() },
            ClientError::Transport,
        ),
        (
            SendError::Interrupted { text: text() },
            ClientError::Transport,
        ),
    ];
    for (error, expected) in rows {
        assert_eq!(send_failure(&error), expected, "{error:?}");
    }
}

#[test]
fn given_a_protocol_failure_before_the_headers_when_mapped_then_it_is_malformed() {
    let mapped = send_failure(&SendError::Protocol { text: text() });
    assert!(matches!(mapped, ClientError::MalformedResponse { .. }));
}

#[test]
fn given_a_failure_of_the_body_when_mapped_then_it_follows_the_generic_table() {
    for error in [
        BodyError::Interrupted { text: text() },
        BodyError::IdleTimeout { text: text() },
        BodyError::TotalTimeout { text: text() },
    ] {
        assert_eq!(body_failure(&error), ClientError::StreamTruncated);
    }
    for error in [
        BodyError::UnsupportedEncoding { text: text() },
        BodyError::Protocol { text: text() },
    ] {
        assert!(matches!(
            body_failure(&error),
            ClientError::MalformedResponse { .. }
        ));
    }
}

#[test]
fn given_a_failure_of_the_sse_decoder_when_mapped_then_only_a_truncated_input_is_a_truncated_stream()
 {
    assert_eq!(
        decode_failure(&DecodeError::InputTruncated),
        ClientError::StreamTruncated
    );
    for error in [
        DecodeError::InvalidLimit,
        DecodeError::FrameTooLarge { limit: 10 },
        DecodeError::InvalidUtf8,
        DecodeError::InputEnded,
    ] {
        assert!(matches!(
            decode_failure(&error),
            ClientError::MalformedResponse { .. }
        ));
    }
}
