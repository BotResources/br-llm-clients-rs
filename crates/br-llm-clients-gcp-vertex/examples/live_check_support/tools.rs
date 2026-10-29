use br_llm_clients_core::{OutputMode, Parameter, ParameterKind, Parameters, ToolSpec};
use br_llm_clients_gcp_vertex::VertexClient;
use br_llm_messages::{BlockId, RuntimeToolCall, RuntimeToolResult, ToolName};
use serde_json::Value;

use crate::checks::{answered, require};
use crate::fail;
use crate::report::has_text;
use crate::sink::CountingSink;
use crate::thread::{Thread, plain_text};

const MAX_ROUNDS: usize = 4;
const SYSTEM: &str = "Use the tool get_weather to know the weather. Never guess it.";

pub async fn tool_call(client: &VertexClient) -> Result<(), String> {
    let tool = weather_tool()?;
    let mut thread = Thread::new()?;
    thread.say("What is the weather in Paris? Call get_weather, then answer in one sentence.")?;
    let mut executed = 0usize;
    for _ in 0..MAX_ROUNDS {
        let request = thread.request(Some(SYSTEM), vec![tool.clone()], OutputMode::Plain)?;
        let response = answered(client, &request, &mut CountingSink::default()).await?;
        let results = response
            .runtime_calls()
            .map(fake_weather)
            .collect::<Result<Vec<_>, _>>()?;
        let final_text = has_text(&response);
        thread.record(response)?;
        if results.is_empty() {
            println!("  the tool ran {executed} time(s)");
            require(executed > 0, "the model never called the tool")?;
            return require(final_text, "the final reply holds no text");
        }
        executed = executed.saturating_add(results.len());
        thread.deliver(results)?;
    }
    Err(format!("no final reply after {MAX_ROUNDS} rounds"))
}

fn weather_tool() -> Result<ToolSpec, String> {
    let name = ToolName::new("get_weather").map_err(fail)?;
    let city = Parameter::new("city", "The name of the city", true, ParameterKind::String)
        .map_err(fail)?;
    let parameters = Parameters::new(vec![city]).map_err(fail)?;
    ToolSpec::new(name, "Gives the current weather of a city", parameters).map_err(fail)
}

fn fake_weather(call: &RuntimeToolCall) -> Result<RuntimeToolResult, String> {
    let arguments = call.arguments().deserialize_into::<Value>().map_err(fail)?;
    let city = arguments
        .get("city")
        .and_then(Value::as_str)
        .unwrap_or("an unknown city");
    let report = format!("{city}: 18 degrees Celsius, partly cloudy");
    RuntimeToolResult::new(
        BlockId::mint(),
        call.id(),
        call.name().clone(),
        false,
        vec![plain_text(&report)?],
    )
    .map_err(fail)
}
