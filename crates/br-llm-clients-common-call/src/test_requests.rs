use br_llm_clients_core::Parameters;
use br_llm_clients_core::{ClientRequest, OutputMode, StructuredShape, ToolSpec};
use br_llm_messages::{
    AssistantBlock, Text, ToolCall, ToolCallId, ToolName, ToolResult, WireMessage, WireUserBlock,
};

fn text(value: &str) -> Text {
    Text::new(value).expect("text")
}

pub(crate) fn request_with_calls(ids: &[&str]) -> ClientRequest {
    let name = ToolName::new("search").expect("name");
    let calls = ids
        .iter()
        .map(|id| {
            AssistantBlock::ToolCall(ToolCall {
                id: ToolCallId::new(*id).expect("id"),
                name: name.clone(),
                arguments: serde_json::json!({}),
            })
        })
        .collect();
    let results = ids
        .iter()
        .map(|id| {
            WireUserBlock::ToolResult(ToolResult::new(
                ToolCallId::new(*id).expect("id"),
                name.clone(),
                Vec::new(),
                false,
            ))
        })
        .collect();
    ClientRequest::new(
        None,
        vec![
            WireMessage::User {
                content: vec![WireUserBlock::Text {
                    text: text("hello"),
                }],
            },
            WireMessage::Assistant { content: calls },
            WireMessage::User { content: results },
        ],
        Vec::new(),
        OutputMode::Plain,
    )
    .expect("request")
}

fn tool() -> ToolSpec {
    let name = ToolName::new("search").expect("name");
    ToolSpec::new(name, "", Parameters::new(Vec::new()).expect("parameters")).expect("tool")
}

pub(crate) fn structured_request(with_tools: bool) -> ClientRequest {
    let shape = StructuredShape::new("shape", None, Parameters::new(Vec::new()).expect("p"))
        .expect("shape");
    let tools = if with_tools { vec![tool()] } else { Vec::new() };
    ClientRequest::new(
        None,
        vec![WireMessage::User {
            content: vec![WireUserBlock::Text {
                text: text("hello"),
            }],
        }],
        tools,
        OutputMode::Structured(shape),
    )
    .expect("request")
}
