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

mod block;
mod call_record;
mod call_spec;
mod classify;
mod conclude;
mod dialect;
mod emitter;
mod encoded_request;
mod error_clean;
mod error_info;
mod failure_map;
mod generated_id;
mod judge;
mod normalizer;
mod outcome;
mod payload;
mod payload_decoder;
mod provider_event;
mod read_reply;
mod reader;
mod refusal;
mod request_build;
mod run;
mod text_clean;

pub use br_llm_clients_common_http::{Authorizer, Endpoint, Headers, HttpClient, Secret};
pub use call_spec::CallSpec;
pub use dialect::Dialect;
pub use encoded_request::EncodedRequest;
pub use error_info::ErrorInfo;
pub use generated_id::is_generated_id;
pub use payload::{Payload, PayloadError};
pub use payload_decoder::{End, PayloadDecoder};
pub use provider_event::{ProviderBlock, ProviderEvent};
pub use run::run;

#[cfg(test)]
mod classify_stream_tests;
#[cfg(test)]
mod classify_tests;
#[cfg(test)]
mod emitter_tests;
#[cfg(test)]
mod error_clean_tests;
#[cfg(test)]
mod failure_map_tests;
#[cfg(test)]
mod judge_tests;
#[cfg(test)]
mod normalizer_close_tests;
#[cfg(test)]
mod normalizer_holdback_tests;
#[cfg(test)]
mod normalizer_ids_tests;
#[cfg(test)]
mod normalizer_order_tests;
#[cfg(test)]
mod normalizer_rules_tests;
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
mod stream_conclude_tests;
#[cfg(test)]
mod stream_end_tests;
#[cfg(test)]
mod stream_failure_tests;
#[cfg(test)]
mod stream_finish_tests;
#[cfg(test)]
mod stream_report_tests;
#[cfg(test)]
mod stream_split_tests;
#[cfg(test)]
mod stream_wire_tests;
#[cfg(test)]
mod test_decoder;
#[cfg(test)]
mod test_dialect;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod test_normalize;
#[cfg(test)]
mod test_requests;
#[cfg(test)]
mod test_server;
#[cfg(test)]
mod test_sink;
#[cfg(test)]
mod test_stream;
#[cfg(test)]
mod text_clean_tests;
