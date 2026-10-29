use br_llm_clients_core::{ClientRequest, OutputMode, Parameters, StructuredShape, ToolSpec};
use br_llm_messages::ToolName;

use crate::test_fixtures::request_with;

fn tool() -> ToolSpec {
    let name = ToolName::new("search").expect("name");
    ToolSpec::new(name, "", Parameters::default()).expect("tool")
}

pub(crate) fn structured_request(with_tools: bool) -> ClientRequest {
    let shape = StructuredShape::new("shape", None, Parameters::default()).expect("shape");
    let tools = if with_tools { vec![tool()] } else { Vec::new() };
    request_with(tools, OutputMode::Structured(shape))
}
