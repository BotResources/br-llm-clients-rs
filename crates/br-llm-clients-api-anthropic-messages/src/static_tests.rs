use std::sync::Arc;

use br_llm_clients_common_call::{Dialect, Payload};

use crate::test_decode::decoder;
use crate::test_fixtures::dialect;

#[test]
fn given_the_dialect_when_shared_as_a_trait_object_then_it_answers() {
    let shared: Arc<dyn Dialect> = Arc::new(dialect());
    assert!(shared.error_code("{}").is_none());
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
