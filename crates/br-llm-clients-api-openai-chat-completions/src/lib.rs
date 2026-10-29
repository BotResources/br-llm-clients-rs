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

mod choice;
mod decoder;
mod delta;
mod dialect;
mod encode;
mod encode_assistant;
mod encode_messages;
mod encode_output;
mod encode_parts;
mod encode_plan;
mod encode_schema;
mod encode_tools;
mod encode_user;
mod ending;
mod error_code;
mod finish;
mod json_fields;
mod max_tokens_field;
mod options;
mod options_error;
mod reasoning_effort;
mod reasoning_rule;
mod reasoning_table;
mod request_id;
mod stream_error;
mod tool_fragments;
mod tool_only_content;
mod usage;

pub use br_llm_clients_common_call::{
    Dialect, DialectHooks, EncodedRequest, End, ErrorInfo, Headers, Layered, Payload,
    PayloadDecoder, PayloadError, ProviderBlock, ProviderEvent,
};
pub use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    ReasoningLevel, RenderedHistory,
};
pub use br_llm_messages::{ModelName, ResponseEnding, Usage};
pub use dialect::ChatCompletionsDialect;
pub use max_tokens_field::MaxTokensField;
pub use options::ChatCompletionsOptions;
pub use options_error::OptionsError;
pub use reasoning_effort::ReasoningEffort;
pub use reasoning_rule::ReasoningRule;
pub use reasoning_table::ReasoningTable;
pub use serde_json::{Map, Value};
pub use tool_only_content::ToolOnlyContent;

#[cfg(test)]
mod decode_content_tests;
#[cfg(test)]
mod decode_error_tests;
#[cfg(test)]
mod decode_finish_tests;
#[cfg(test)]
mod decode_model_tests;
#[cfg(test)]
mod decode_reasoning_tests;
#[cfg(test)]
mod decode_shape_tests;
#[cfg(test)]
mod decode_tool_fragment_tests;
#[cfg(test)]
mod decode_tool_tests;
#[cfg(test)]
mod decode_usage_cache_tests;
#[cfg(test)]
mod decode_usage_tests;
#[cfg(test)]
mod encode_assistant_tests;
#[cfg(test)]
mod encode_body_tests;
#[cfg(test)]
mod encode_carry_back_tests;
#[cfg(test)]
mod encode_default_bytes_tests;
#[cfg(test)]
mod encode_max_tokens_tests;
#[cfg(test)]
mod encode_merge_tests;
#[cfg(test)]
mod encode_output_tests;
#[cfg(test)]
mod encode_refusal_tests;
#[cfg(test)]
mod encode_tool_switches_tests;
#[cfg(test)]
mod encode_tools_tests;
#[cfg(test)]
mod encode_user_tests;
#[cfg(test)]
mod error_code_tests;
#[cfg(test)]
mod golden_cases;
#[cfg(test)]
mod host_shapes_tests;
#[cfg(test)]
mod options_new_tests;
#[cfg(test)]
mod options_reserved_tests;
#[cfg(test)]
mod options_tests;
#[cfg(test)]
mod reasoning_omit_tests;
#[cfg(test)]
mod reasoning_table_tests;
#[cfg(test)]
mod recorded_decode_tests;
#[cfg(test)]
mod recorded_error_tests;
#[cfg(test)]
mod recorded_fixture_tests;
#[cfg(test)]
mod recorded_run_tests;
#[cfg(test)]
mod recorded_split_tests;
#[cfg(test)]
mod run_failure_tests;
#[cfg(test)]
mod run_request_id_tests;
#[cfg(test)]
mod run_support;
#[cfg(test)]
mod run_text_tests;
#[cfg(test)]
mod run_timing_tests;
#[cfg(test)]
mod run_tool_tests;
#[cfg(test)]
mod run_truncation_tests;
#[cfg(test)]
mod static_tests;
#[cfg(test)]
mod test_recordings;
#[cfg(test)]
mod test_server;
#[cfg(test)]
mod test_sink;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod test_thread;
#[cfg(test)]
mod wrap_tests;
