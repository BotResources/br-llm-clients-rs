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

mod acquire;
mod announced;
mod authorize;
mod authorizer;
mod authorizer_error;
mod authorizers;
mod body;
mod body_error;
mod body_state;
mod claim;
mod classify;
mod client;
mod config;
mod config_error;
mod connect;
mod deadline;
mod dial;
mod dialer;
mod driver;
mod encoding;
mod endpoint;
mod endpoint_parse;
mod endpoint_scheme;
mod exchange;
mod flow;
mod h2state;
mod h2track;
mod header;
mod headers;
mod host;
mod idle;
mod lease;
mod limits;
mod payload;
mod percent;
mod pool;
#[cfg(test)]
mod pool_hooks;
mod prepare;
mod redact;
mod request;
mod response;
mod retry_after;
mod secret;
mod send_error;
mod server_fault;
mod settings_watch;
mod settled;
mod spy_io;
mod texts;
mod tls;
mod transport;
mod tuning;
mod unsent;

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
mod acquire_deadline_tests;
#[cfg(test)]
mod acquire_race_tests;
#[cfg(test)]
mod acquire_tests;
#[cfg(test)]
mod acquire_zero_tests;
#[cfg(test)]
mod alpn_cap_tests;
#[cfg(test)]
mod announce_steps_tests;
#[cfg(test)]
mod authorize_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod debug_tests;
#[cfg(test)]
mod dial_panic_tests;
#[cfg(test)]
mod dial_tests;
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
mod payload_tests;
#[cfg(test)]
mod plan_settle_tests;
#[cfg(test)]
mod pool_tests;
#[cfg(test)]
mod request_tests;
#[cfg(test)]
mod retry_after_tests;
#[cfg(test)]
mod secret_tests;
#[cfg(test)]
mod server_fault_tests;
#[cfg(test)]
mod settled_tests;
#[cfg(test)]
mod spy_io_model_tests;
#[cfg(test)]
mod spy_io_tests;
#[cfg(test)]
mod sweep_tests;
#[cfg(test)]
mod turns_tests;

#[cfg(test)]
extern crate self as br_llm_clients_common_http;
#[cfg(test)]
mod bump_storm_tests;
#[cfg(test)]
mod cap_tests;
#[cfg(test)]
mod classify_tests;
#[cfg(test)]
mod driver_tests;
#[cfg(test)]
mod exchange_tests;
#[cfg(test)]
mod fixed_values_tests;
#[cfg(test)]
mod h2cap_tests;
#[cfg(test)]
mod h2state_tests;
#[cfg(test)]
mod host_tests;
#[cfg(test)]
mod hot_readers_tests;
#[cfg(test)]
mod idle_cap_tests;
#[cfg(test)]
mod lifetime_tests;
#[cfg(test)]
mod ready_tests;
#[cfg(test)]
#[allow(dead_code)] // the end-to-end helpers are shared with this build, which uses a subset
#[path = "../tests/e2e/support/mod.rs"]
mod support;
