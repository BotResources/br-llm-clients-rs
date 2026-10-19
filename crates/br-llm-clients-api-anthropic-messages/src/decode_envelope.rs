use br_llm_clients_common_call::{ErrorInfo, Payload, PayloadError};
use serde_json::Value;

use crate::decode_fields::{Fields, malformed};

pub(crate) enum Envelope {
    Error(ErrorInfo),
    Event { kind: String, fields: Fields },
}

pub(crate) fn read(payload: &Payload) -> Result<Envelope, PayloadError> {
    let parsed = serde_json::from_str::<Value>(payload.data()).ok();
    let announces_error = payload.event() == Some("error")
        || parsed
            .as_ref()
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
            == Some("error");
    if announces_error {
        return Ok(Envelope::Error(error_info(payload.data(), parsed.as_ref())));
    }
    let Some(Value::Object(fields)) = parsed else {
        return Err(malformed("the data of the payload is not a JSON object"));
    };
    let Some(kind) = fields
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Err(malformed("the data of the payload has no text type"));
    };
    if payload.event().is_some_and(|event| event != kind) {
        return Err(malformed(
            "the event name of the payload differs from the type of its data",
        ));
    }
    Ok(Envelope::Event { kind, fields })
}

fn error_info(data: &str, parsed: Option<&Value>) -> ErrorInfo {
    let info = ErrorInfo::new(data);
    let code = parsed
        .and_then(|value| value.get("error"))
        .and_then(|error| error.get("type"))
        .and_then(Value::as_str);
    match code {
        Some(code) => info.with_code(code),
        None => info,
    }
}
