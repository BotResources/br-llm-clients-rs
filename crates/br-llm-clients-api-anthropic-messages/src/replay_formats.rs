use br_llm_clients_common_call::PayloadError;
use br_llm_messages::{ReplayFormat, ReplayMaterial, ReplayPayload};

pub(crate) const SIGNATURE_FORMAT: &str = "br-llm-clients-api-anthropic-messages/signature";
pub(crate) const REDACTED_DATA_FORMAT: &str = "br-llm-clients-api-anthropic-messages/redacted-data";

pub(crate) fn replay_format(name: &str) -> Result<ReplayFormat, PayloadError> {
    ReplayFormat::new(name).map_err(|_| PayloadError::new("a replay format cannot be named"))
}

pub(crate) fn text_in_format<'a>(material: &'a ReplayMaterial, name: &str) -> Option<&'a str> {
    if material.format().as_str() != name {
        return None;
    }
    match material.payload() {
        ReplayPayload::Text { value, .. } => Some(value.as_str()),
        _ => None,
    }
}
