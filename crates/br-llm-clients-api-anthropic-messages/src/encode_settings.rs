use br_llm_clients_core::{CacheRequirement, ClientError, ClientSettings, ReasoningLevel};
use serde_json::{Value, json};

pub(crate) struct EncodeSettings {
    model: String,
    max_tokens: u32,
    reasoning: ReasoningLevel,
    cache: CacheRequirement,
}

impl EncodeSettings {
    pub(crate) fn from_settings(settings: &ClientSettings) -> Self {
        Self {
            model: settings.model().as_str().to_owned(),
            max_tokens: settings.max_output_tokens().get(),
            reasoning: settings.reasoning(),
            cache: settings.cache(),
        }
    }

    pub(crate) fn model(&self) -> &str {
        &self.model
    }

    pub(crate) fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    pub(crate) fn reasoning(&self) -> ReasoningLevel {
        self.reasoning
    }

    pub(crate) fn cache(&self) -> CacheRequirement {
        self.cache
    }
}

pub(crate) struct Thinking {
    pub(crate) field: Value,
    pub(crate) effort: Option<&'static str>,
}

pub(crate) fn thinking(level: ReasoningLevel) -> Result<Thinking, ClientError> {
    let adaptive = |effort| Thinking {
        field: json!({"type": "adaptive"}),
        effort: Some(effort),
    };
    match level {
        ReasoningLevel::Off => Ok(Thinking {
            field: json!({"type": "disabled"}),
            effort: None,
        }),
        ReasoningLevel::Low => Ok(adaptive("low")),
        ReasoningLevel::Medium => Ok(adaptive("medium")),
        ReasoningLevel::High => Ok(adaptive("high")),
        ReasoningLevel::Extra => Ok(adaptive("xhigh")),
        ReasoningLevel::Max => Ok(adaptive("max")),
        _ => Err(ClientError::ReasoningNotSupported {
            level,
            reason: "this version of the dialect does not know this reasoning level".to_owned(),
        }),
    }
}

pub(crate) fn cache_control(cache: CacheRequirement) -> Result<Option<Value>, ClientError> {
    match cache {
        CacheRequirement::Required => Ok(Some(json!({"type": "ephemeral"}))),
        CacheRequirement::NotRequired => Ok(None),
        _ => Err(ClientError::CacheNotSupported),
    }
}
