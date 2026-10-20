use std::fmt;

use serde_json::{Map, Value};

use crate::options_error::OptionsError;

const RESERVED_MEMBERS: [&str; 10] = [
    "model",
    "max_tokens",
    "stream",
    "messages",
    "system",
    "tools",
    "thinking",
    "output_config",
    "cache_control",
    "anthropic_version",
];

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModelPlacement {
    Body,
    Omitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VersionPlacement {
    Header,
    Body(String),
}

#[derive(Clone, PartialEq)]
pub struct MessagesOptions {
    model: ModelPlacement,
    version: VersionPlacement,
    extra_body: Map<String, Value>,
}

impl MessagesOptions {
    pub fn new() -> Self {
        Self {
            model: ModelPlacement::Body,
            version: VersionPlacement::Header,
            extra_body: Map::new(),
        }
    }

    pub fn with_model_placement(mut self, placement: ModelPlacement) -> Self {
        self.model = placement;
        self
    }

    pub fn with_version_placement(mut self, placement: VersionPlacement) -> Self {
        self.version = placement;
        self
    }

    pub fn with_extra_body(mut self, extra_body: Map<String, Value>) -> Result<Self, OptionsError> {
        if let Some(member) = RESERVED_MEMBERS
            .iter()
            .find(|member| extra_body.contains_key(**member))
        {
            return Err(OptionsError::ReservedExtraBodyMember((*member).to_owned()));
        }
        self.extra_body = extra_body;
        Ok(self)
    }

    pub(crate) fn model(&self) -> &ModelPlacement {
        &self.model
    }

    pub(crate) fn version(&self) -> &VersionPlacement {
        &self.version
    }

    pub(crate) fn extra_body(&self) -> &Map<String, Value> {
        &self.extra_body
    }
}

impl Default for MessagesOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for MessagesOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessagesOptions")
            .field("model", &self.model)
            .field("version", &self.version)
            .field(
                "extra_body_members",
                &self.extra_body.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}
