use br_llm_clients_core::{Parameter, ParameterKind, Parameters, ToolSpec};
use br_llm_messages::{
    BlockId, EntryId, ModelResponse, PlainBlock, RuntimeToolCall, RuntimeToolResult,
    RuntimeToolResults, TextBlock, ToolName, TurnEntry,
};
use serde_json::Value;

use crate::Failure;

pub fn weather_tool() -> Result<ToolSpec, Failure> {
    let city = Parameter::new("city", "The name of the city", true, ParameterKind::String)?;
    Ok(ToolSpec::new(
        ToolName::new("get_weather")?,
        "Gives the current weather of one city",
        Parameters::new(vec![city])?,
    )?)
}

fn fake_result(tool_call: &RuntimeToolCall) -> Result<RuntimeToolResult, Failure> {
    let arguments: Value = tool_call.arguments().deserialize_into()?;
    let city = arguments
        .get("city")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    println!("[tools] fake execution of get_weather for {city}");
    let text = TextBlock::new(BlockId::mint(), format!("{city}: 18 degrees, cloudy"))?;
    Ok(RuntimeToolResult::new(
        BlockId::mint(),
        tool_call.id(),
        tool_call.name().clone(),
        false,
        vec![PlainBlock::Text(text)],
    )?)
}

pub fn results_entry(response: &ModelResponse) -> Result<Option<(usize, TurnEntry)>, Failure> {
    let results = response
        .runtime_calls()
        .map(fake_result)
        .collect::<Result<Vec<_>, _>>()?;
    if results.is_empty() {
        return Ok(None);
    }
    let count = results.len();
    let entry = TurnEntry::RuntimeToolResults(RuntimeToolResults::new(EntryId::mint(), results)?);
    Ok(Some((count, entry)))
}
