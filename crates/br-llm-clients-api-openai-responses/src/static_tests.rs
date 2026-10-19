use crate::{OpenAiResponses, Options};

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn given_the_dialect_when_checked_then_it_is_send_and_sync() {
    assert_send_sync::<OpenAiResponses>();
    assert_send_sync::<Options>();
}

#[test]
fn given_a_default_options_when_read_then_the_summary_is_auto_and_the_parameter_is_sent() {
    let options = Options::default();
    assert_eq!(options.reasoning_summary, crate::ReasoningSummary::Auto);
    assert_eq!(options.reasoning_parameter, crate::ReasoningParameter::Sent);
}
