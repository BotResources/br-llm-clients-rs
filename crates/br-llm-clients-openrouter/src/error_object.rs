use serde_json::{Map, Value};

const MIN_STATUS: u64 = 400;
const MAX_STATUS: u64 = 599;
const MODERATION_MEMBERS: [&str; 2] = ["reasons", "flagged_input"];

pub(crate) struct ErrorObject {
    pub(crate) error_type: Option<String>,
    pub(crate) code: Option<String>,
    pub(crate) status: Option<u16>,
    pub(crate) limit_source: Option<String>,
    pub(crate) moderation: bool,
}

fn text_of(object: &Map<String, Value>, name: &str) -> Option<String> {
    match object.get(name) {
        Some(Value::String(text)) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

fn status_of(number: u64) -> Option<u16> {
    (MIN_STATUS..=MAX_STATUS)
        .contains(&number)
        .then(|| u16::try_from(number).ok())
        .flatten()
}

fn code_of(object: &Map<String, Value>) -> (Option<String>, Option<u16>) {
    match object.get("code") {
        Some(Value::Number(number)) => (None, number.as_u64().and_then(status_of)),
        Some(Value::String(text)) if !text.is_empty() => {
            let status = text
                .bytes()
                .all(|byte| byte.is_ascii_digit())
                .then(|| text.parse::<u64>().ok().and_then(status_of))
                .flatten();
            (Some(text.clone()), status)
        }
        _ => (None, None),
    }
}

struct Parts {
    object: Map<String, Value>,
    outer_type: Option<String>,
    outer_metadata: Option<Map<String, Value>>,
}

fn object_member(object: &mut Map<String, Value>, name: &str) -> Option<Map<String, Value>> {
    match object.remove(name) {
        Some(Value::Object(member)) => Some(member),
        _ => None,
    }
}

fn split(mut root: Map<String, Value>) -> Option<Parts> {
    let outer_type = text_of(&root, "error_type");
    if let Some(object) = object_member(&mut root, "error") {
        return Some(Parts {
            object,
            outer_type,
            outer_metadata: object_member(&mut root, "metadata"),
        });
    }
    matches!(root.get("message"), Some(Value::String(_))).then_some(Parts {
        object: root,
        outer_type,
        outer_metadata: None,
    })
}

pub(crate) fn read(text: &str) -> Option<ErrorObject> {
    let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text) else {
        return None;
    };
    let Parts {
        mut object,
        outer_type,
        outer_metadata,
    } = split(root)?;
    let metadata = object_member(&mut object, "metadata").or(outer_metadata);
    let metadata = metadata.as_ref();
    let error_type = metadata
        .and_then(|metadata| text_of(metadata, "error_type"))
        .or_else(|| text_of(&object, "error_type"))
        .or(outer_type);
    let limit_source = metadata.and_then(|metadata| text_of(metadata, "limit_source"));
    let moderation = metadata.is_some_and(|metadata| {
        MODERATION_MEMBERS
            .iter()
            .any(|member| metadata.contains_key(*member))
    });
    let (code, status) = code_of(&object);
    Some(ErrorObject {
        error_type,
        code,
        status,
        limit_source,
        moderation,
    })
}
