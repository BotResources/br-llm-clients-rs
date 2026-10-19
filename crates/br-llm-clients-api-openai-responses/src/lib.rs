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

mod decode;
mod dialect;
mod encode;
mod envelope;
mod error_code;
mod error_envelope;
mod input;
mod input_assistant;
mod input_user;
mod options;
mod reasoning_plan;
mod refine;
mod request_id;
mod schema;
mod structured;
mod tools;
mod turn;

pub use dialect::OpenAiResponses;
pub use options::{Options, ReasoningParameter, ReasoningSummary};

#[cfg(test)]
mod encode_assistant_tests;
#[cfg(test)]
mod encode_body_tests;
#[cfg(test)]
mod encode_golden_tests;
#[cfg(test)]
mod encode_reasoning_tests;
#[cfg(test)]
mod encode_structured_tests;
#[cfg(test)]
mod encode_tools_tests;
#[cfg(test)]
mod encode_user_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod static_tests;
#[cfg(test)]
mod test_requests;
