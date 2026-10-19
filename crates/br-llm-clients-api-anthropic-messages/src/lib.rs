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
