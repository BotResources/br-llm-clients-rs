#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic
)]

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilitiesError {
    #[error("images are supported but the set of image media types is empty")]
    InconsistentCapabilities,
}
