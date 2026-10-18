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

mod call_record;
mod call_spec;
mod classify;
mod dialect;
mod encoded_request;
mod error_clean;
mod error_info;
mod failure_map;
mod outcome;
mod payload;
mod payload_decoder;
mod provider_event;
mod read_reply;
mod refusal;
mod request_build;
mod run;
mod text_clean;

pub use br_llm_clients_common_http::{Authorizer, Endpoint, Headers, HttpClient, Secret};
pub use call_spec::CallSpec;
pub use dialect::Dialect;
pub use encoded_request::EncodedRequest;
pub use error_info::ErrorInfo;
pub use payload::{Payload, PayloadError};
pub use payload_decoder::{End, PayloadDecoder};
pub use provider_event::{ProviderBlock, ProviderEvent};
pub use run::run;

#[cfg(test)]
mod classify_stream_tests;
#[cfg(test)]
mod classify_tests;
#[cfg(test)]
mod error_clean_tests;
#[cfg(test)]
mod failure_map_tests;
#[cfg(test)]
mod outcome_tests;
#[cfg(test)]
mod run_guard_tests;
#[cfg(test)]
mod run_refusal_tests;
#[cfg(test)]
mod run_send_tests;
#[cfg(test)]
mod run_timing_tests;
#[cfg(test)]
mod static_tests;
#[cfg(test)]
mod test_dialect;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod test_server;
#[cfg(test)]
mod text_clean_tests;
