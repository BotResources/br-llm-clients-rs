#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic
)]

use br_llm_messages::MediaType;

use crate::accepted::Accepted;
use crate::capabilities_error::CapabilitiesError;
use crate::capabilities_spec::CapabilitiesSpec;
use crate::max_output_tokens::MaxOutputTokens;
use crate::reasoning::ReasoningLevel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    spec: CapabilitiesSpec,
}

impl Capabilities {
    pub fn new(spec: CapabilitiesSpec) -> Result<Self, CapabilitiesError> {
        if spec.is_inconsistent() {
            return Err(CapabilitiesError::InconsistentCapabilities);
        }
        Ok(Self { spec })
    }

    pub fn normalized(spec: CapabilitiesSpec) -> Self {
        let images = spec.images && !spec.is_inconsistent();
        Self {
            spec: CapabilitiesSpec { images, ..spec },
        }
    }

    pub fn everything() -> Self {
        Self::normalized(CapabilitiesSpec::everything())
    }

    pub fn nothing() -> Self {
        Self::normalized(CapabilitiesSpec::nothing())
    }

    pub fn spec(&self) -> &CapabilitiesSpec {
        &self.spec
    }

    pub fn reasoning_levels(&self) -> &Accepted<ReasoningLevel> {
        &self.spec.reasoning_levels
    }

    pub fn image_media_types(&self) -> &Accepted<MediaType> {
        &self.spec.image_media_types
    }

    pub fn max_output_tokens(&self) -> MaxOutputTokens {
        self.spec.max_output_tokens
    }

    pub fn tools(&self) -> bool {
        self.spec.tools
    }

    pub fn structured_output(&self) -> bool {
        self.spec.structured_output
    }

    pub fn structured_output_with_tools(&self) -> bool {
        self.spec.structured_output_with_tools
    }

    pub fn caching(&self) -> bool {
        self.spec.caching
    }

    pub fn images(&self) -> bool {
        self.spec.images
    }

    pub fn accepts_structured_output_with_tools(&self) -> bool {
        self.spec.tools && self.spec.structured_output && self.spec.structured_output_with_tools
    }
}
