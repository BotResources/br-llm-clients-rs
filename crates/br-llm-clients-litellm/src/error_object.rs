use serde_json::{Map, Value};

const MIN_STATUS: u64 = 400;
const MAX_STATUS: u64 = 599;

pub(crate) struct ErrorObject {
    pub(crate) kind: Option<String>,
    pub(crate) status: Option<u16>,
}

fn object_of(mut root: Map<String, Value>) -> Option<Map<String, Value>> {
    if let Some(Value::Object(error)) = root.remove("error") {
        return Some(error);
    }
    matches!(root.get("message"), Some(Value::String(_))).then_some(root)
}

fn kind_of(object: &Map<String, Value>) -> Option<String> {
    match object.get("type") {
        Some(Value::String(kind)) if !kind.is_empty() && kind != "None" => Some(kind.clone()),
        _ => None,
    }
}

fn status_of(object: &Map<String, Value>) -> Option<u16> {
    let number = match object.get("code")? {
        Value::Number(number) => number.as_u64()?,
        Value::String(text)
            if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            text.parse::<u64>().ok()?
        }
        _ => return None,
    };
    if (MIN_STATUS..=MAX_STATUS).contains(&number) {
        u16::try_from(number).ok()
    } else {
        None
    }
}

pub(crate) fn read(text: &str) -> Option<ErrorObject> {
    let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text) else {
        return None;
    };
    let object = object_of(root)?;
    Some(ErrorObject {
        kind: kind_of(&object),
        status: status_of(&object),
    })
}
