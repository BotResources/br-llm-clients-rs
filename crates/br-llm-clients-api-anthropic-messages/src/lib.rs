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
mod decode_blocks;
mod decode_envelope;
mod decode_fields;
mod decode_message;
mod decode_stop;
mod decode_usage;
mod dialect;
mod encode;
mod encode_blocks;
mod encode_messages;
mod encode_schema;
mod encode_settings;
mod encode_tools;
mod errors;

pub use dialect::AnthropicMessages;

#[cfg(test)]
mod decode_block_delta_tests;
#[cfg(test)]
mod decode_block_start_tests;
#[cfg(test)]
mod decode_hold_tests;
#[cfg(test)]
mod decode_message_tests;
#[cfg(test)]
mod decode_shape_tests;
#[cfg(test)]
mod decode_stop_tests;
#[cfg(test)]
mod decode_stream_tests;
#[cfg(test)]
mod decode_usage_tests;
#[cfg(test)]
mod encode_envelope_tests;
#[cfg(test)]
mod encode_messages_tests;
#[cfg(test)]
mod encode_reasoning_tests;
#[cfg(test)]
mod encode_refusal_tests;
#[cfg(test)]
mod encode_structured_tests;
#[cfg(test)]
mod encode_thinking_tests;
#[cfg(test)]
mod encode_tools_tests;
#[cfg(test)]
mod error_code_tests;
#[cfg(test)]
mod refine_rules_tests;
#[cfg(test)]
mod refine_table_tests;
#[cfg(test)]
mod run_combined_tests;
#[cfg(test)]
mod run_malformed_tests;
#[cfg(test)]
mod run_refusal_tests;
#[cfg(test)]
mod run_request_tests;
#[cfg(test)]
mod run_stream_tests;
#[cfg(test)]
mod run_truncation_tests;
#[cfg(test)]
mod static_tests;
#[cfg(test)]
mod test_call;
#[cfg(test)]
mod test_decode;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod test_recordings;
#[cfg(test)]
mod test_server;
#[cfg(test)]
mod test_sink;
#[cfg(test)]
mod test_streams;
#[cfg(test)]
mod test_streams_constructed;
