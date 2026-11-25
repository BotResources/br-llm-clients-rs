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

mod aws_credentials;
mod build_error;
mod capabilities;
mod client;
mod config;
mod credential;
mod hooks;
mod region;
mod route;
mod sigv4;
mod visible;

pub use aws_credentials::{AwsCredentials, CredentialSource, StaticCredentials};
pub use br_llm_clients_common_call::{Endpoint, HttpClient, Secret};
pub use br_llm_clients_common_http::{AuthorizerError, BoxFuture};
pub use build_error::BedrockBuildError;
pub use client::BedrockClient;
pub use config::BedrockConfig;
pub use credential::BedrockCredential;
