use br_llm_messages::ModelName;

use crate::cache::CacheRequirement;
use crate::max_output_tokens::MaxOutputTokens;
use crate::reasoning::ReasoningLevel;
use crate::schema::{self, LoadError};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(into = "StoredClientSettings")]
pub struct ClientSettings {
    model: ModelName,
    reasoning: ReasoningLevel,
    cache: CacheRequirement,
    max_output_tokens: MaxOutputTokens,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredClientSettings {
    schema: String,
    model: ModelName,
    reasoning: ReasoningLevel,
    cache: CacheRequirement,
    max_output_tokens: MaxOutputTokens,
}

impl TryFrom<StoredClientSettings> for ClientSettings {
    type Error = LoadError;

    fn try_from(stored: StoredClientSettings) -> Result<Self, Self::Error> {
        schema::check(&stored.schema)?;
        Ok(Self {
            model: stored.model,
            reasoning: stored.reasoning,
            cache: stored.cache,
            max_output_tokens: stored.max_output_tokens,
        })
    }
}

impl<'de> serde::Deserialize<'de> for ClientSettings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let raw = schema::checked_first(deserializer)?;
        let stored = StoredClientSettings::deserialize(raw).map_err(D::Error::custom)?;
        Self::try_from(stored).map_err(D::Error::custom)
    }
}

impl From<ClientSettings> for StoredClientSettings {
    fn from(settings: ClientSettings) -> Self {
        Self {
            schema: schema::SCHEMA_VERSION.to_owned(),
            model: settings.model,
            reasoning: settings.reasoning,
            cache: settings.cache,
            max_output_tokens: settings.max_output_tokens,
        }
    }
}

impl ClientSettings {
    pub fn new(
        model: ModelName,
        reasoning: ReasoningLevel,
        cache: CacheRequirement,
        max_output_tokens: MaxOutputTokens,
    ) -> Self {
        Self {
            model,
            reasoning,
            cache,
            max_output_tokens,
        }
    }

    pub fn model(&self) -> &ModelName {
        &self.model
    }

    pub fn reasoning(&self) -> ReasoningLevel {
        self.reasoning
    }

    pub fn cache(&self) -> CacheRequirement {
        self.cache
    }

    pub fn max_output_tokens(&self) -> MaxOutputTokens {
        self.max_output_tokens
    }
}
