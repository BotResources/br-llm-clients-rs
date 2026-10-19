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
mod encode_schema;
mod encode_tools;
mod encode_user;
mod error_code;
mod finish;
mod json_fields;
mod options;
mod options_error;
mod reasoning_effort;
mod reasoning_rule;
mod reasoning_table;
mod request_id;
mod stop_reason;
mod stream_error;
mod tool_fragments;
mod usage;

pub use br_llm_clients_common_call::{
    Dialect, EncodedRequest, End, ErrorInfo, Headers, Payload, PayloadDecoder, PayloadError,
    ProviderBlock, ProviderEvent,
};
pub use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, MaxOutputTokens, OutputMode,
    ReasoningLevel,
};
pub use br_llm_messages::{ModelId, StopReason, Usage};
pub use dialect::ChatCompletionsDialect;
pub use options::ChatCompletionsOptions;
pub use options_error::OptionsError;
pub use reasoning_effort::ReasoningEffort;
pub use reasoning_rule::ReasoningRule;
pub use reasoning_table::ReasoningTable;
pub use serde_json::{Map, Value};

#[cfg(test)]
mod encode_assistant_tests;
#[cfg(test)]
mod encode_body_tests;
#[cfg(test)]
mod encode_output_tests;
#[cfg(test)]
mod encode_refusal_tests;
#[cfg(test)]
mod encode_tools_tests;
#[cfg(test)]
mod encode_user_tests;
#[cfg(test)]
mod options_tests;
#[cfg(test)]
mod reasoning_table_tests;
#[cfg(test)]
mod test_support;
