#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]

mod authorize;
mod authorizer;
mod authorizer_error;
mod authorizers;
mod body;
mod body_error;
mod body_state;
mod classify;
mod client;
mod config;
mod config_error;
mod connect;
mod deadline;
mod encoding;
mod endpoint;
mod endpoint_parse;
mod endpoint_scheme;
mod exchange;
mod flow;
mod header;
mod headers;
mod lease;
mod payload;
mod percent;
mod pool;
mod prepare;
mod redact;
mod request;
mod response;
mod retry_after;
mod secret;
mod send_error;
mod tls;
mod transport;

pub use authorizer::{AuthorizationRequest, AuthorizedHeader, Authorizer, BoxFuture};
pub use authorizer_error::AuthorizerError;
pub use authorizers::{ApiKeyAuthorizer, BearerAuthorizer, NoAuthorizer};
pub use body::{BodyStream, Collected};
pub use body_error::BodyError;
pub use client::HttpClient;
pub use config::HttpConfig;
pub use config_error::ConfigError;
pub use endpoint::Endpoint;
pub use headers::Headers;
pub use request::Request;
pub use response::Response;
pub use secret::Secret;
pub use send_error::SendError;

#[cfg(test)]
mod authorize_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod debug_tests;
#[cfg(test)]
mod encoding_tests;
#[cfg(test)]
mod endpoint_build_tests;
#[cfg(test)]
mod endpoint_host_tests;
#[cfg(test)]
mod endpoint_tests;
#[cfg(test)]
mod headers_tests;
#[cfg(test)]
mod request_tests;
#[cfg(test)]
mod retry_after_tests;
#[cfg(test)]
mod secret_tests;
