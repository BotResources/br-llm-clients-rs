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
mod client;
mod config;
mod hooks;
mod options;
mod presets;
mod profile;
mod refine;
mod route;
mod validate;

pub use br_llm_clients_api_openai_responses::{
    Map, PromptCacheMode, ReasoningParameter, ReasoningSummary, ServiceTier, Value,
};
pub use br_llm_clients_core::{
    Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec, IdSource,
};
pub use build_error::OpenAiBuildError;
pub use client::OpenAiClient;
pub use config::OpenAiConfig;
pub use profile::ModelProfile;
