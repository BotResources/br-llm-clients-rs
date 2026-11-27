use br_llm_clients_api_anthropic_messages::{MessagesCacheOptions, OptionsError};
use serde_json::Value;

pub const CACHE_ENV: &str = "AWS_BEDROCK_CACHE";

const MESSAGES: &str = "messages";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BedrockCache {
    messages: MessagesCacheOptions,
}

impl BedrockCache {
    pub fn new(messages: MessagesCacheOptions) -> Self {
        Self { messages }
    }

    pub fn parse(text: Option<&str>) -> Result<Self, BedrockCacheError> {
        let Some(text) = text.filter(|text| !text.trim().is_empty()) else {
            return Ok(Self::default());
        };
        let value: Value = serde_json::from_str(text).map_err(|_| BedrockCacheError::NotJson)?;
        let Value::Object(members) = value else {
            return Err(BedrockCacheError::NotAnObject);
        };
        if let Some(unknown) = members.keys().find(|member| member.as_str() != MESSAGES) {
            return Err(BedrockCacheError::UnknownMember {
                member: unknown.clone(),
            });
        }
        let messages = match members.get(MESSAGES) {
            None => MessagesCacheOptions::default(),
            Some(section) => MessagesCacheOptions::from_json(section).map_err(refused_messages)?,
        };
        Ok(Self { messages })
    }

    pub fn messages(&self) -> MessagesCacheOptions {
        self.messages
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BedrockCacheError {
    #[error("{} is not JSON", CACHE_ENV)]
    NotJson,
    #[error("{} is not a JSON object", CACHE_ENV)]
    NotAnObject,
    #[error("{} holds the member `{member}`, which is not known", CACHE_ENV)]
    UnknownMember { member: String },
    #[error("{} is refused at the member `{}`", CACHE_ENV, messages_member(.member))]
    InvalidMessages { member: Option<String> },
}

fn refused_messages(error: OptionsError) -> BedrockCacheError {
    let member = match error {
        OptionsError::InvalidCacheOption { member } => member,
        _ => None,
    };
    BedrockCacheError::InvalidMessages { member }
}

fn messages_member(member: &Option<String>) -> String {
    match member {
        Some(member) => format!("{MESSAGES}.{member}"),
        None => MESSAGES.to_owned(),
    }
}
