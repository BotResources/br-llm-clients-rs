use std::sync::Arc;

use br_llm_clients_common_call::{Dialect, Payload, PayloadDecoder};

use crate::AnthropicMessages;
use crate::test_decode::decoder;
use crate::test_fixtures::dialect;

fn assert_send_sync<T: Send + Sync>() {}

fn assert_send<T: Send>() {}

#[test]
fn given_the_dialect_when_checked_then_it_is_send_sync_and_object_safe() {
    assert_send_sync::<AnthropicMessages>();
    let shared: Arc<dyn Dialect> = Arc::new(dialect());
    assert_send_sync::<Arc<dyn Dialect>>();
    assert!(shared.error_code("{}").is_none());
}

#[test]
fn given_a_decoder_when_checked_then_it_is_send() {
    assert_send::<Box<dyn PayloadDecoder>>();
}

#[test]
fn given_one_dialect_when_two_decoders_are_made_then_their_state_is_separate() {
    let mut first = decoder(false);
    let mut second = decoder(false);
    first
        .payload(Payload::new(
            None,
            r#"{"type":"message_start","message":{}}"#,
        ))
        .expect("started");
    assert!(
        second
            .payload(Payload::new(None, r#"{"type":"message_stop"}"#))
            .is_err()
    );
    assert!(
        first
            .payload(Payload::new(None, r#"{"type":"message_stop"}"#))
            .is_ok()
    );
}

#[test]
fn given_a_dialect_when_cloned_then_both_encode_the_same_bytes() {
    let original = dialect();
    let copy = original.clone();
    let request = crate::test_fixtures::request(vec![crate::test_fixtures::user("hi")]);
    assert_eq!(
        original.encode(&request).expect("original"),
        copy.encode(&request).expect("copy")
    );
}
