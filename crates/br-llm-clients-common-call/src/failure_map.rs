use br_llm_clients_common_http::{AuthorizerError, BodyError, SendError};
use br_llm_clients_common_sse::DecodeError;
use br_llm_clients_core::ClientError;
use br_llm_messages::Error as MessageError;

use crate::payload::PayloadError;

pub(crate) fn malformed(detail: &str) -> ClientError {
    ClientError::MalformedResponse {
        detail: detail.to_owned(),
    }
}

pub(crate) fn send_failure(error: &SendError) -> ClientError {
    match error {
        SendError::Authorizer(AuthorizerError::Unavailable { .. })
        | SendError::InvalidAuthorization { .. } => ClientError::Authentication,
        SendError::Protocol { .. } => malformed("the server did not answer in valid HTTP"),
        _ => ClientError::Transport,
    }
}

pub(crate) fn body_failure(error: &BodyError) -> ClientError {
    match error {
        BodyError::UnsupportedEncoding { .. } => {
            malformed("the response body uses a content encoding that is not supported")
        }
        BodyError::Protocol { .. } => malformed("the response body is not valid HTTP"),
        _ => ClientError::StreamTruncated,
    }
}

pub(crate) fn decode_failure(error: &DecodeError) -> ClientError {
    match error {
        DecodeError::InputTruncated => ClientError::StreamTruncated,
        other => malformed(&format!("the event stream cannot be read: {other}")),
    }
}

pub(crate) fn payload_failure(error: &PayloadError) -> ClientError {
    malformed(error.text())
}

pub(crate) fn message_failure(error: &MessageError) -> ClientError {
    malformed(&error.to_string())
}
