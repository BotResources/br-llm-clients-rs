use serde_json::{Map, Value};

pub(crate) fn text_part(text: &str) -> Value {
    let mut part = Map::new();
    part.insert("type".to_owned(), Value::from("text"));
    part.insert("text".to_owned(), Value::from(text));
    Value::Object(part)
}

pub(crate) fn message(role: &str) -> Map<String, Value> {
    let mut message = Map::new();
    message.insert("role".to_owned(), Value::from(role));
    message
}
