use crate::endpoint::Endpoint;
use crate::redact::Redactor;
use crate::send_error::SendError;
use crate::server_fault::protocol_failure;

#[test]
fn given_a_server_fault_when_built_then_the_text_names_the_endpoint_and_hides_secrets() {
    let endpoint = Endpoint::https("https://h.example:8443/v1?token=q").expect("endpoint");
    let redactor = Redactor::new(["s3cr3t"]);
    let error = protocol_failure(&redactor, &endpoint, "the fault s3cr3t");
    let SendError::Protocol { text } = error else {
        panic!("{error:?}");
    };
    assert_eq!(text, "https://h.example:8443/v1: the fault [redacted]");
}
