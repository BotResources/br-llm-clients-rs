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
mod credential;
mod dialect_options;
mod hooks;
mod kimi_thinking;
mod rewrite;
mod rewrite_names;
mod rewrite_reasoning;
mod route;

pub use build_error::KimiBuildError;
pub use client::KimiClient;
pub use config::KimiConfig;
pub use kimi_thinking::KimiThinking;
