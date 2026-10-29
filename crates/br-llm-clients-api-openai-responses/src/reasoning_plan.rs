use br_llm_clients_core::{ClientError, ReasoningLevel};
use serde_json::{Map, Value, json};

use crate::options::{Options, ReasoningParameter};

pub(crate) struct ReasoningFields {
    pub(crate) reasoning: Option<Value>,
    pub(crate) include: Option<Value>,
}

fn effort(level: ReasoningLevel) -> Option<&'static str> {
    match level {
        ReasoningLevel::Off => Some("none"),
        ReasoningLevel::Low => Some("low"),
        ReasoningLevel::Medium => Some("medium"),
        ReasoningLevel::High => Some("high"),
        ReasoningLevel::Extra => Some("xhigh"),
        ReasoningLevel::Max => Some("max"),
        _ => None,
    }
}

fn not_supported(level: ReasoningLevel, reason: &str) -> ClientError {
    ClientError::ReasoningNotSupported {
        level,
        reason: reason.to_owned(),
    }
}

pub(crate) fn plan(
    level: ReasoningLevel,
    options: &Options,
) -> Result<ReasoningFields, ClientError> {
    let none = ReasoningFields {
        reasoning: None,
        include: None,
    };
    if options.reasoning_parameter == ReasoningParameter::Omitted {
        return if level == ReasoningLevel::Off {
            Ok(none)
        } else {
            Err(not_supported(
                level,
                "the model takes no reasoning parameter",
            ))
        };
    }
    let Some(effort) = effort(level) else {
        return Err(not_supported(
            level,
            "this version of the dialect does not know the level",
        ));
    };
    let mut reasoning = Map::new();
    reasoning.insert("effort".to_owned(), Value::String(effort.to_owned()));
    if level == ReasoningLevel::Off {
        return Ok(ReasoningFields {
            reasoning: Some(Value::Object(reasoning)),
            include: None,
        });
    }
    if let Some(summary) = options.reasoning_summary.wire() {
        reasoning.insert("summary".to_owned(), Value::String(summary.to_owned()));
    }
    Ok(ReasoningFields {
        reasoning: Some(Value::Object(reasoning)),
        include: Some(json!(["reasoning.encrypted_content"])),
    })
}
