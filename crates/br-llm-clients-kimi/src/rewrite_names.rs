use std::collections::HashMap;

use br_llm_messages::{WireMessage, WireUserBlock};
use serde_json::Value;

use br_llm_clients_core::ClientRequest;

pub(crate) fn name_tool_messages(messages: &mut [Value], request: &ClientRequest) {
    let names = tool_names(request);
    for message in messages.iter_mut().filter_map(Value::as_object_mut) {
        if message.get("role").and_then(Value::as_str) != Some("tool") {
            continue;
        }
        let name = message
            .get("tool_call_id")
            .and_then(Value::as_str)
            .and_then(|id| names.get(id));
        if let Some(name) = name {
            message.insert("name".to_owned(), Value::from(*name));
        }
    }
}

fn tool_names(request: &ClientRequest) -> HashMap<&str, &str> {
    let mut names = HashMap::new();
    for message in request.messages() {
        let WireMessage::User { content } = message else {
            continue;
        };
        for block in content {
            if let WireUserBlock::ToolResult(result) = block {
                names.insert(result.tool_call_id.as_str(), result.tool_name.as_str());
            }
        }
    }
    names
}
