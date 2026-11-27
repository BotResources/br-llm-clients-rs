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
mod cache;
mod capabilities;
mod client;
mod config;
mod credential;
mod hooks;

pub use br_llm_clients_api_anthropic_messages::{CacheTtl, MessagesCacheOptions};
pub use build_error::AnthropicBuildError;
pub use cache::{AnthropicCache, AnthropicCacheError, CACHE_ENV};
pub use client::AnthropicClient;
pub use config::AnthropicConfig;
