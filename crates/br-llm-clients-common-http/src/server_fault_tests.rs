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
    assert_eq!(text, "the fault [redacted] (https://h.example:8443/v1)");
}

#[test]
fn given_a_very_long_path_when_a_server_fault_is_built_then_the_reason_is_not_crowded_out() {
    let path = format!("/{}", "p".repeat(8000));
    let endpoint = Endpoint::https(&format!("https://h.example{path}")).expect("endpoint");
    let error = protocol_failure(&Redactor::default(), &endpoint, "the server broke a rule");
    let SendError::Protocol { text } = error else {
        panic!("{error:?}");
    };
    assert!(
        text.starts_with("the server broke a rule (https://h.example/ppp"),
        "{text}"
    );
    assert!(text.chars().count() <= 512, "{}", text.chars().count());
}
