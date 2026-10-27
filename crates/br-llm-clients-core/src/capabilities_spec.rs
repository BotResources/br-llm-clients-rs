#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic
)]

use std::num::NonZeroU32;

use br_llm_messages::MediaType;

use crate::accepted::Accepted;
use crate::max_output_tokens::MaxOutputTokens;
use crate::reasoning::ReasoningLevel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitiesSpec {
    pub reasoning_levels: Accepted<ReasoningLevel>,
    pub max_output_tokens: MaxOutputTokens,
    pub tools: bool,
    pub structured_output: bool,
    pub structured_output_with_tools: bool,
    pub caching: bool,
    pub images: bool,
    pub image_media_types: Accepted<MediaType>,
}

impl CapabilitiesSpec {
    pub fn everything() -> Self {
        Self {
            reasoning_levels: Accepted::Any,
            max_output_tokens: Self::max_output_tokens_of(NonZeroU32::MAX),
            tools: true,
            structured_output: true,
            structured_output_with_tools: true,
            caching: true,
            images: true,
            image_media_types: Accepted::Any,
        }
    }

    pub fn nothing() -> Self {
        Self {
            reasoning_levels: Accepted::Only(vec![ReasoningLevel::Off]),
            max_output_tokens: Self::max_output_tokens_of(NonZeroU32::MAX),
            tools: false,
            structured_output: false,
            structured_output_with_tools: false,
            caching: false,
            images: false,
            image_media_types: Accepted::Only(Vec::new()),
        }
    }

    pub fn max_output_tokens_of(limit: NonZeroU32) -> MaxOutputTokens {
        MaxOutputTokens::from_non_zero(limit)
    }

    pub(crate) fn is_inconsistent(&self) -> bool {
        self.images && self.image_media_types.is_empty_set()
    }
}
