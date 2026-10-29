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

mod assertion;
mod build_error;
mod capabilities;
mod client;
mod config;
mod credential;
mod google_token;
mod hooks;
mod key_fields;
mod location;
mod project;
mod route;
mod service_account_key;

pub use br_llm_clients_common_call::{Endpoint, HttpClient};
pub use build_error::VertexBuildError;
pub use client::VertexClient;
pub use config::VertexConfig;
pub use credential::VertexCredential;
pub use service_account_key::ServiceAccountKey;
