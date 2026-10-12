use br_llm_clients_core::{MaxOutputTokens, ReasoningLevel};
use br_llm_messages::ImageMime;

use crate::accepted::Accepted;
use crate::capabilities_error::CapabilitiesError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitiesSpec {
    pub reasoning_levels: Accepted<ReasoningLevel>,
    pub max_output_tokens: MaxOutputTokens,
    pub tools: bool,
    pub structured_output: bool,
    pub structured_output_with_tools: bool,
    pub caching: bool,
    pub images: bool,
    pub image_media_types: Accepted<ImageMime>,
}

impl CapabilitiesSpec {
    pub fn everything() -> Self {
        Self {
            reasoning_levels: Accepted::Any,
            max_output_tokens: largest_output_tokens(),
            tools: true,
            structured_output: true,
            structured_output_with_tools: true,
            caching: true,
            images: true,
            image_media_types: Accepted::Any,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    spec: CapabilitiesSpec,
}

impl Capabilities {
    pub fn new(spec: CapabilitiesSpec) -> Result<Self, CapabilitiesError> {
        if spec.images && spec.image_media_types.is_empty_set() {
            return Err(CapabilitiesError::InconsistentCapabilities);
        }
        Ok(Self { spec })
    }

    pub fn everything() -> Self {
        Self {
            spec: CapabilitiesSpec::everything(),
        }
    }

    pub fn spec(&self) -> &CapabilitiesSpec {
        &self.spec
    }

    pub(crate) fn reasoning_levels(&self) -> &Accepted<ReasoningLevel> {
        &self.spec.reasoning_levels
    }

    pub(crate) fn max_output_tokens(&self) -> MaxOutputTokens {
        self.spec.max_output_tokens
    }

    pub(crate) fn tools(&self) -> bool {
        self.spec.tools
    }

    pub(crate) fn structured_output(&self) -> bool {
        self.spec.structured_output
    }

    pub(crate) fn caching(&self) -> bool {
        self.spec.caching
    }

    pub(crate) fn images(&self) -> bool {
        self.spec.images
    }

    pub(crate) fn image_media_types(&self) -> &Accepted<ImageMime> {
        &self.spec.image_media_types
    }

    pub(crate) fn accepts_structured_output_with_tools(&self) -> bool {
        self.spec.tools && self.spec.structured_output && self.spec.structured_output_with_tools
    }
}

#[allow(
    clippy::expect_used,
    reason = "u32::MAX is above zero, the only value MaxOutputTokens refuses"
)]
fn largest_output_tokens() -> MaxOutputTokens {
    MaxOutputTokens::new(u32::MAX).expect("u32::MAX is above zero")
}
