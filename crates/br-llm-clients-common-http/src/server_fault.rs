use crate::endpoint::Endpoint;
use crate::redact::Redactor;
use crate::send_error::SendError;

pub(crate) fn protocol_failure(redactor: &Redactor, endpoint: &Endpoint, what: &str) -> SendError {
    SendError::Protocol {
        text: redactor.finish(&format!("{endpoint}: {what}")),
    }
}
