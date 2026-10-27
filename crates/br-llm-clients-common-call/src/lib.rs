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

mod answered_model;
mod block;
mod call_record;
mod call_spec;
mod classify;
mod conclude;
mod dialect;
mod dialect_hooks;
mod emitter;
mod encoded_request;
mod error_clean;
mod error_info;
mod failure_map;
mod judge;
mod layered;
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
mod secret_log;
mod text_clean;

pub use br_llm_clients_common_http::{Authorizer, Endpoint, Headers, HttpClient, Secret};
pub use call_spec::CallSpec;
pub use dialect::Dialect;
pub use dialect_hooks::DialectHooks;
pub use encoded_request::EncodedRequest;
pub use error_info::ErrorInfo;
pub use layered::Layered;
pub use payload::{Payload, PayloadError};
pub use payload_decoder::{End, PayloadDecoder};
pub use provider_event::{ProviderBlock, ProviderEvent};
pub use run::run;
pub use secret_log::SecretLog;
#[cfg(test)]
mod answered_model_tests;
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
mod layered_run_tests;
#[cfg(test)]
mod layered_tests;
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
mod normalizer_thinking_tests;
#[cfg(test)]
mod outcome_tests;
#[cfg(test)]
mod request_id_guard_tests;
#[cfg(test)]
mod run_cut_secret_tests;
#[cfg(test)]
mod run_guard_tests;
#[cfg(test)]
mod run_refusal_tests;
#[cfg(test)]
mod run_retry_after_tests;
#[cfg(test)]
mod run_secret_log_tests;
#[cfg(test)]
mod run_send_tests;
#[cfg(test)]
mod run_timing_tests;
#[cfg(test)]
mod secret_log_tests;
#[cfg(test)]
mod secret_trim_tests;
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
mod stream_ids_tests;
#[cfg(test)]
mod stream_report_tests;
#[cfg(test)]
mod stream_split_tests;
#[cfg(test)]
mod stream_wire_tests;
#[cfg(test)]
mod test_authorizer;
#[cfg(test)]
mod test_decoder;
#[cfg(test)]
mod test_dialect;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod test_ids;
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
