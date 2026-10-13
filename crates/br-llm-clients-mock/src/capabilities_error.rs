#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilitiesError {
    #[error("images are supported but the set of image media types is empty")]
    InconsistentCapabilities,
}
