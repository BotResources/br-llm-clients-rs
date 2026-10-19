use std::sync::Arc;

use br_llm_clients_common_call::{Dialect, PayloadDecoder};
use br_llm_clients_core::OutputMode;
use serde_json::json;

use crate::test_support::{dialect, encoded_body, feed, hello, request_of, user_text};
use crate::{
    ChatCompletionsDialect, ChatCompletionsOptions, OptionsError, ReasoningEffort, ReasoningRule,
    ReasoningTable,
};

fn is_send_and_sync<T: Send + Sync>() {}

fn is_send<T: Send>() {}

#[test]
fn given_the_public_types_when_checked_then_the_dialect_is_send_and_sync_and_usable_as_a_trait_object()
 {
    is_send_and_sync::<ChatCompletionsDialect>();
    is_send_and_sync::<ChatCompletionsOptions>();
    is_send_and_sync::<ReasoningTable>();
    is_send_and_sync::<OptionsError>();
    is_send::<Box<dyn PayloadDecoder>>();
    let shared: Arc<dyn Dialect> = Arc::new(dialect());
    assert!(shared.encode(&hello()).is_ok());
}

#[test]
fn given_the_public_value_types_when_used_then_they_have_debug_clone_and_equality() {
    let table = ReasoningTable::openai_effort();
    assert_eq!(table.clone(), table);
    assert!(!format!("{table:?}").is_empty());
    let rule = ReasoningRule::Send(ReasoningEffort::High);
    assert_eq!(rule, rule);
    let options = ChatCompletionsOptions::new(table);
    assert_eq!(options.clone(), options);
    assert!(!format!("{options:?}").is_empty());
}

#[tokio::test]
async fn given_one_dialect_when_many_calls_use_it_at_once_then_each_gets_its_own_result() {
    let shared = Arc::new(dialect());
    let mut tasks = Vec::new();
    for number in 0..16 {
        let dialect = Arc::clone(&shared);
        tasks.push(tokio::spawn(async move {
            let request = request_of(vec![user_text(&format!("message {number}"))]);
            let body = encoded_body(&dialect, &request);
            let mut decoder = dialect.decoder(&OutputMode::Plain);
            let data =
                json!({"choices": [{"delta": {"content": format!("reply {number}")}}]}).to_string();
            let events = feed(decoder.as_mut(), &data).expect("chunk");
            (number, body, events.len())
        }));
    }
    for task in tasks {
        let (number, body, events) = task.await.expect("task");
        assert_eq!(
            body["messages"][0]["content"],
            json!(format!("message {number}"))
        );
        assert_eq!(events, 2);
    }
}

#[test]
fn given_two_decoders_of_one_dialect_when_fed_then_their_state_is_separate() {
    let dialect = dialect();
    let mut first = dialect.decoder(&OutputMode::Plain);
    let mut second = dialect.decoder(&OutputMode::Plain);
    let finish = json!({"choices": [{"delta": {}, "finish_reason": "stop"}]}).to_string();
    assert!(feed(first.as_mut(), &finish).is_ok());
    assert!(feed(second.as_mut(), &finish).is_ok());
    assert!(feed(first.as_mut(), &finish).is_err());
}
