use br_llm_messages::ModelId;

use crate::cache::CacheRequirement;
use crate::max_output_tokens::MaxOutputTokens;
use crate::reasoning::ReasoningLevel;
use crate::schema::{self, LoadError};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "StoredClientSettings", into = "StoredClientSettings")]
pub struct ClientSettings {
    model: ModelId,
    reasoning: ReasoningLevel,
    cache: CacheRequirement,
    max_output_tokens: MaxOutputTokens,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredClientSettings {
    schema: String,
    model: ModelId,
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

impl From<ClientSettings> for StoredClientSettings {
    fn from(settings: ClientSettings) -> Self {
        Self {
            schema: schema::current(),
            model: settings.model,
            reasoning: settings.reasoning,
            cache: settings.cache,
            max_output_tokens: settings.max_output_tokens,
        }
    }
}

impl ClientSettings {
    pub fn new(
        model: ModelId,
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

    pub fn model(&self) -> &ModelId {
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
