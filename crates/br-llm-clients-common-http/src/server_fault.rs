use crate::endpoint::Endpoint;
use crate::redact::Redactor;
use crate::send_error::SendError;

pub(crate) fn about_endpoint(redactor: &Redactor, endpoint: &Endpoint, what: &str) -> String {
    redactor.finish(&format!("{what} ({endpoint})"))
}

pub(crate) fn protocol_failure(redactor: &Redactor, endpoint: &Endpoint, what: &str) -> SendError {
    SendError::Protocol {
        text: about_endpoint(redactor, endpoint, what),
    }
}
