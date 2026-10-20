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

mod api_family;
mod build_error;
mod check_key;
mod client;
mod config;
mod error_object;
mod extra_headers;
mod hooks;
mod key_header;
mod key_placement;
mod refine;
mod route;
mod secrets;

pub use api_family::ApiFamily;
pub use br_llm_clients_api_openai_chat_completions::{
    ChatCompletionsOptions, Map, ReasoningEffort, ReasoningRule, ReasoningTable, Value,
};
pub use br_llm_clients_api_openai_responses::{Options, ReasoningParameter, ReasoningSummary};
pub use br_llm_clients_common_http::{Endpoint, HttpClient, Secret};
pub use br_llm_clients_core::{
    Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec, ReasoningLevel,
};
pub use build_error::LiteLlmBuildError;
pub use client::LiteLlmClient;
pub use config::LiteLlmConfig;
pub use key_placement::KeyPlacement;
