mod build_error;
mod cache;
mod call_outcome;
mod call_report;
mod client;
mod client_error;
mod client_request;
mod client_settings;
mod max_output_tokens;
mod message_rules;
mod output_mode;
mod reasoning;
mod schema;
mod sink;
mod tool;

#[cfg(test)]
mod call_outcome_tests;
#[cfg(test)]
mod client_error_tests;
#[cfg(test)]
mod client_request_tests;
#[cfg(test)]
mod client_settings_tests;
#[cfg(test)]
mod client_trait_tests;
#[cfg(test)]
mod message_rules_tests;
#[cfg(test)]
mod test_support;

pub use build_error::BuildError;
pub use cache::CacheRequirement;
pub use call_outcome::CallOutcome;
pub use call_report::CallReport;
pub use client::{BoxFuture, Client};
pub use client_error::{ClientError, LimitKind};
pub use client_request::ClientRequest;
pub use client_settings::ClientSettings;
pub use max_output_tokens::MaxOutputTokens;
pub use output_mode::{OutputMode, StructuredShape};
pub use reasoning::ReasoningLevel;
pub use schema::SCHEMA_VERSION;
pub use sink::StreamSink;
pub use tool::{MAX_NESTING_DEPTH, Parameter, ParameterKind, Parameters, ToolSpec};
