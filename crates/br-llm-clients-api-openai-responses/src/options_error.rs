use std::fmt;

use serde_json::{Map, Value};

const RESERVED_MEMBERS: [&str; 13] = [
    "model",
    "stream",
    "store",
    "instructions",
    "input",
    "max_output_tokens",
    "tools",
    "text",
    "reasoning",
    "include",
    "prompt_cache_key",
    "service_tier",
    "prompt_cache_options",
];

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OptionsError {
    ReservedExtraBodyMember(String),
}

impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReservedExtraBodyMember(member) => {
                write!(f, "the extra body member `{member}` is reserved")
            }
        }
    }
}

impl std::error::Error for OptionsError {}

pub(crate) fn reserved_member(extra_body: &Map<String, Value>) -> Option<String> {
    RESERVED_MEMBERS
        .iter()
        .find(|member| extra_body.contains_key(**member))
        .map(|member| (*member).to_owned())
}
