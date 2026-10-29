#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::panic
    )
)]

mod build_error;
mod capabilities;
mod client;
mod config;
mod credential;
mod hooks;

pub use build_error::AnthropicBuildError;
pub use client::AnthropicClient;
pub use config::AnthropicConfig;
