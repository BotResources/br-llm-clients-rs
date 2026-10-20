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

mod build_error;
mod capabilities;
mod client;
mod config;
mod hooks;
mod visible;

pub use build_error::AnthropicBuildError;
pub use client::AnthropicClient;
pub use config::AnthropicConfig;
