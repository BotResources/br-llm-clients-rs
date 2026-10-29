use br_llm_clients_common_call::ProviderEvent;

use crate::test_decode::*;

#[test]
fn given_a_message_start_with_a_model_an_id_and_usage_when_decoded_then_they_come_in_that_order() {
    let mut session = Session::plain();
    let events = session.ok(
        r#"{"type":"message_start","message":{"usage":{"input_tokens":4,"output_tokens":1},"id":"msg_x","model":"claude-x"}}"#,
    );
    assert_eq!(
        events,
        vec![
            ProviderEvent::Model("claude-x".to_owned()),
            ProviderEvent::ResponseId("msg_x".to_owned()),
            ProviderEvent::Usage(usage(4, 1)),
        ]
    );
}

#[test]
fn given_a_message_start_without_a_usable_id_when_decoded_then_no_response_id_is_given() {
    for message in [
        r#"{"model":"m"}"#,
        r#"{"model":"m","id":""}"#,
        r#"{"model":"m","id":5}"#,
        r#"{"model":"m","id":null}"#,
        r#"{"model":"m","id":{"a":1}}"#,
    ] {
        let mut session = Session::plain();
        let events = session.ok(&format!(
            r#"{{"type":"message_start","message":{message}}}"#
        ));
        assert_eq!(
            events,
            vec![ProviderEvent::Model("m".to_owned())],
            "{message}"
        );
    }
}

#[test]
fn given_a_message_start_with_an_id_and_no_model_when_decoded_then_only_the_response_id_is_given() {
    let mut session = Session::plain();
    let events = session.ok(r#"{"type":"message_start","message":{"id":"msg_y"}}"#);
    assert_eq!(events, vec![ProviderEvent::ResponseId("msg_y".to_owned())]);
}
