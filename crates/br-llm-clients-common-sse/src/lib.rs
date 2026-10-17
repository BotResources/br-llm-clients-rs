#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]

mod decode_error;
mod decoder;
mod frame;
mod line;
mod open_frame;

pub use decode_error::DecodeError;
pub use decoder::{DEFAULT_LIMIT, Decoder};
pub use frame::Frame;

#[cfg(test)]
mod chunking_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod field_tests;
#[cfg(test)]
mod limit_tests;
#[cfg(test)]
mod line_end_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod utf8_tests;
#[cfg(test)]
mod whatwg_tests;
