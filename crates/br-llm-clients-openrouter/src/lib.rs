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
mod headers;
mod hooks;
mod refine;
mod route;
mod stream_chunks;

pub use api_family::ApiFamily;
pub use br_llm_clients_api_anthropic_messages::{
    MessagesOptions, ModelPlacement, OptionsError as MessagesOptionsError, VersionPlacement,
};
pub use br_llm_clients_api_openai_chat_completions::{
    ChatCompletionsOptions, MaxTokensField, OptionsError as ChatCompletionsOptionsError,
    ReasoningEffort, ReasoningRule, ReasoningTable, ToolOnlyContent,
};
pub use br_llm_clients_api_openai_responses::{
    Options, OptionsError as ResponsesOptionsError, PromptCacheMode, ReasoningParameter,
    ReasoningSummary, ServiceTier,
};
pub use br_llm_clients_common_http::{Endpoint, HttpClient, Secret};
pub use br_llm_clients_core::{
    Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec, DefaultIdSource, IdSource,
    ReasoningLevel,
};
pub use build_error::OpenRouterBuildError;
pub use client::OpenRouterClient;
pub use config::{DEFAULT_BASE, OpenRouterConfig};
pub use serde_json::{Map, Value};
