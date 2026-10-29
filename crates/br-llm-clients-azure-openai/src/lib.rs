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

mod api;
mod base;
mod build_error;
mod client;
mod config;
mod dialect;
mod hooks;
mod refine;
mod route;
mod validate;
mod version;

pub use api::AzureOpenAiApi;
pub use br_llm_clients_api_openai_chat_completions::{
    ChatCompletionsOptions, Map, MaxTokensField, ReasoningEffort, ReasoningRule, ReasoningTable,
    ToolOnlyContent, Value,
};
pub use br_llm_clients_api_openai_responses::{
    Options as ResponsesOptions, OptionsError as ResponsesOptionsError, PromptCacheMode,
    ReasoningParameter, ReasoningSummary, ServiceTier,
};
pub use br_llm_clients_core::{
    Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec, IdSource,
};
pub use build_error::AzureOpenAiBuildError;
pub use client::AzureOpenAiClient;
pub use config::AzureOpenAiConfig;
pub use version::AzureOpenAiVersion;
