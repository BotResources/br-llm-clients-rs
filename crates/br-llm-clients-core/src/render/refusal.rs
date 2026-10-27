use crate::client_error::ClientError;

pub(super) fn invalid(reason: impl Into<String>) -> ClientError {
    ClientError::InvalidMessages {
        reason: reason.into(),
    }
}
