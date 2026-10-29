use br_llm_messages::Turn;
use serde_json::{Value, json};

use crate::test_support::{dialect, dialect_for, encode_error, messages_of, options, request_of};
use crate::test_thread::{
    answer, call, call_block, calling, in_progress, notice, redacted, result_of, results, signed,
    texts, thinking, user,
};
use crate::{ClientError, ReasoningLevel};

fn encoded(turns: Vec<Turn>) -> Vec<Value> {
    messages_of(&dialect(), &request_of(turns))
}

#[test]
fn given_an_omitted_assistant_message_between_two_user_messages_when_encoded_then_they_merge_into_one()
 {
    let turns = vec![
        user("go"),
        in_progress(vec![
            answer(vec![thinking("hmm"), redacted()]),
            notice("the runner paused"),
        ]),
    ];
    assert_eq!(
        encoded(turns),
        [json!({"role": "user", "content": [
            {"type": "text", "text": "go"},
            {"type": "text", "text": "<notice>the runner paused</notice>"}
        ]})]
    );
}

#[test]
fn given_an_omitted_assistant_message_after_tool_results_when_encoded_then_the_texts_join_one_user_message()
 {
    let lookup = call("c1", "lookup", json!({}));
    let turns = vec![
        user("go"),
        in_progress(vec![
            calling(vec![call_block(&lookup)]),
            results(vec![result_of(&lookup, texts(&["x"]), false)]),
            answer(vec![signed("sealed")]),
            notice("first"),
        ]),
        user("second"),
    ];
    let messages = encoded(turns);
    assert_eq!(
        messages[2..],
        [
            json!({"role": "tool", "tool_call_id": "c1", "content": "x"}),
            json!({"role": "user", "content": [
                {"type": "text", "text": "<notice>first</notice>"},
                {"type": "text", "text": "second"}
            ]}),
        ]
    );
}

#[test]
fn given_a_last_assistant_message_holding_only_dropped_thinking_when_encoded_then_it_is_omitted_and_not_refused()
 {
    let turns = vec![user("go"), in_progress(vec![answer(vec![thinking("hmm")])])];
    assert_eq!(encoded(turns), [json!({"role": "user", "content": "go"})]);
}

#[test]
fn given_the_carry_back_option_when_an_assistant_message_is_omitted_then_its_thinking_reaches_no_message()
 {
    let configured = options().with_reasoning_carry_back_field("reasoning_content");
    let turns = vec![
        user("go"),
        in_progress(vec![answer(vec![thinking("lost")]), notice("later")]),
    ];
    let messages = messages_of(
        &dialect_for(ReasoningLevel::Off, configured),
        &request_of(turns),
    );
    assert_eq!(messages.len(), 1);
    assert!(!Value::Array(messages).to_string().contains("lost"));
}

#[test]
fn given_a_history_whose_only_assistant_content_is_dropped_and_no_user_message_when_encoded_then_it_is_invalid()
 {
    let turns = vec![in_progress(vec![answer(vec![thinking("alone")])])];
    let request = request_of(turns);
    assert!(matches!(
        encode_error(&dialect(), &request),
        ClientError::InvalidMessages { .. }
    ));
}
