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
mod check_key;
mod client;
mod config;
mod extra_headers;
mod hooks;
mod options;
mod refine;
mod route;

pub use br_llm_clients_api_openai_chat_completions::{
    ReasoningEffort, ReasoningRule, ReasoningTable,
};
pub use br_llm_clients_common_http::{Endpoint, HttpClient, Secret};
pub use br_llm_clients_core::{
    Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec, DefaultIdSource, IdSource,
    ReasoningLevel,
};
pub use build_error::OmlxBuildError;
pub use client::OmlxClient;
pub use config::OmlxConfig;
pub use serde_json::{Map, Value};
