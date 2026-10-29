#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]

mod decode;
mod dialect;
mod encode;
mod envelope;
mod error_code;
mod error_envelope;
mod input;
mod input_assistant;
mod input_user;
mod option_values;
mod options;
mod options_error;
mod reasoning_plan;
mod refine;
mod request_id;
mod schema;
mod structured;
mod tools;

pub use dialect::OpenAiResponses;
pub use option_values::{PromptCacheMode, ServiceTier};
pub use options::{Options, ReasoningParameter, ReasoningSummary};
pub use options_error::OptionsError;
pub use serde_json::{Map, Value};

#[cfg(test)]
mod decode_call_held_tests;
#[cfg(test)]
mod decode_call_tests;
#[cfg(test)]
mod decode_created_tests;
#[cfg(test)]
mod decode_error_event_tests;
#[cfg(test)]
mod decode_error_nested_tests;
#[cfg(test)]
mod decode_item_state_tests;
#[cfg(test)]
mod decode_reasoning_done_tests;
#[cfg(test)]
mod decode_reasoning_summary_tests;
#[cfg(test)]
mod decode_refusal_tests;
#[cfg(test)]
mod decode_robustness_tests;
#[cfg(test)]
mod decode_structured_tests;
#[cfg(test)]
mod decode_terminal_tests;
#[cfg(test)]
mod decode_text_tests;
#[cfg(test)]
mod decode_unexpected_tests;
#[cfg(test)]
mod decode_usage_tests;
#[cfg(test)]
mod encode_assistant_tests;
#[cfg(test)]
mod encode_body_tests;
#[cfg(test)]
mod encode_golden_tests;
#[cfg(test)]
mod encode_options_tests;
#[cfg(test)]
mod encode_reasoning_tests;
#[cfg(test)]
mod encode_structured_tests;
#[cfg(test)]
mod encode_thinking_tests;
#[cfg(test)]
mod encode_tools_tests;
#[cfg(test)]
mod encode_user_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod options_tests;
#[cfg(test)]
mod recorded_decode_tests;
#[cfg(test)]
mod recorded_error_tests;
#[cfg(test)]
mod recorded_registry_tests;
#[cfg(test)]
mod recorded_run_tests;
#[cfg(test)]
mod recorded_split_tests;
#[cfg(test)]
mod run_error_tests;
#[cfg(test)]
mod run_request_id_tests;
#[cfg(test)]
mod run_stream_error_tests;
#[cfg(test)]
mod run_structured_tests;
#[cfg(test)]
mod run_text_tests;
#[cfg(test)]
mod run_tool_tests;
#[cfg(test)]
mod run_truncation_tests;
#[cfg(test)]
mod static_tests;
#[cfg(test)]
mod test_decoder;
#[cfg(test)]
mod test_events;
#[cfg(test)]
mod test_history;
#[cfg(test)]
mod test_recorded;
#[cfg(test)]
mod test_recordings;
#[cfg(test)]
mod test_requests;
#[cfg(test)]
mod test_run;
#[cfg(test)]
mod test_server;
