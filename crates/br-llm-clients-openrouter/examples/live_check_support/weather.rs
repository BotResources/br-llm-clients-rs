use br_llm_clients_core::{OutputMode, Parameter, ParameterKind, Parameters, ToolSpec};
use br_llm_clients_openrouter::{OpenRouterClient, Value};
use br_llm_messages::{
    BlockId, EntryId, PlainBlock, RuntimeToolCall, RuntimeToolResult, RuntimeToolResults,
    TextBlock, ToolName, TurnEntry,
};

use crate::Failure;
use crate::checks::call;
use crate::show;
use crate::thread::Thread;

const SYSTEM: &str = "You must call get_weather once for each city, then answer.";
const QUESTION: &str = "What is the weather in Paris and in Tokyo?";
const LABEL: &str = "4 tool call";
const MAX_ROUNDS: usize = 5;
const EXPECTED_CALLS: usize = 2;

fn weather_tool() -> Result<ToolSpec, Failure> {
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
    println!("[{LABEL}] fake execution of get_weather for {city}");
    let text = TextBlock::new(BlockId::mint(), format!("{city}: 18 degrees, cloudy"))?;
    Ok(RuntimeToolResult::new(
        BlockId::mint(),
        tool_call.id(),
        tool_call.name().clone(),
        false,
        vec![PlainBlock::Text(text)],
    )?)
}

pub async fn weather(client: &OpenRouterClient) -> Result<bool, Failure> {
    let mut thread = Thread::new(QUESTION)?;
    let mut executed = 0usize;
    for round in 1..=MAX_ROUNDS {
        let request = thread.request(Some(SYSTEM), vec![weather_tool()?], OutputMode::Plain)?;
        let (outcome, _) = call(client, &format!("{LABEL}, round {round}"), &request).await;
        let Ok(response) = outcome.result() else {
            return Ok(false);
        };
        let results = response
            .runtime_calls()
            .map(fake_result)
            .collect::<Result<Vec<_>, _>>()?;
        if results.is_empty() {
            println!("[{LABEL}] tool calls executed: {executed}");
            return Ok(executed >= EXPECTED_CALLS && show::has_text(response));
        }
        executed = executed.saturating_add(results.len());
        thread.push(TurnEntry::ModelResponse(response.clone()));
        thread.push(TurnEntry::RuntimeToolResults(RuntimeToolResults::new(
            EntryId::mint(),
            results,
        )?));
    }
    println!("[{LABEL}] no final answer after {MAX_ROUNDS} rounds");
    Ok(false)
}
