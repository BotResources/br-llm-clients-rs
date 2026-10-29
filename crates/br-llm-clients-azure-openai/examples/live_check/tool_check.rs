use br_llm_clients_azure_openai::{AzureOpenAiClient, Value};
use br_llm_clients_core::{Client, Parameter, ParameterKind, Parameters, ToolSpec};
use br_llm_messages::{
    BlockId, EntryId, RuntimeToolCall, RuntimeToolResult, RuntimeToolResults, ToolName, TurnEntry,
};

use crate::Failure;
use crate::sink::{QuietSink, answer_text};
use crate::thread::{Thread, text_block};

const MAX_ROUNDS: usize = 4;
const SYSTEM: &str = "Always use the weather tool before you speak about the weather.";
const QUESTION: &str = "What is the weather in Paris right now? Answer in one sentence.";
const TOOL: &str = "get_weather";

fn weather_tool() -> Result<ToolSpec, Failure> {
    let city = Parameter::new("city", "the name of the city", true, ParameterKind::String)?;
    Ok(ToolSpec::new(
        ToolName::new(TOOL)?,
        "Gives the current weather of a city.",
        Parameters::new(vec![city])?,
    )?)
}

fn city_of(call: &RuntimeToolCall) -> String {
    call.arguments()
        .deserialize_into::<Value>()
        .ok()
        .and_then(|arguments| {
            arguments
                .get("city")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "an unknown city".to_owned())
}

fn fake_result(call: &RuntimeToolCall) -> Result<RuntimeToolResult, Failure> {
    let city = city_of(call);
    println!("  tool call {} for {city}", call.name().as_str());
    let text = format!("{city}: 18 degrees Celsius, partly cloudy");
    Ok(RuntimeToolResult::new(
        BlockId::mint(),
        call.id(),
        call.name().clone(),
        false,
        vec![text_block(&text)?],
    )?)
}

pub async fn weather(client: &AzureOpenAiClient) -> Result<bool, Failure> {
    let mut thread = Thread::new(QUESTION)?;
    let tool = weather_tool()?;
    let mut executed = 0usize;
    for round in 1..=MAX_ROUNDS {
        println!("  round {round}");
        let request = thread.request(SYSTEM, vec![tool.clone()])?;
        let outcome = client.call(&request, &mut QuietSink).await;
        let response = match outcome.into_parts().1 {
            Ok(response) => response,
            Err(error) => {
                println!("  error: {error}");
                return Ok(false);
            }
        };
        let results = response
            .runtime_calls()
            .map(fake_result)
            .collect::<Result<Vec<_>, _>>()?;
        if results.is_empty() {
            let answer = answer_text(&response);
            println!("  answer: {answer}");
            return Ok(executed > 0 && !answer.trim().is_empty());
        }
        executed = executed.saturating_add(results.len());
        thread.push(TurnEntry::ModelResponse(response));
        thread.push(TurnEntry::RuntimeToolResults(RuntimeToolResults::new(
            EntryId::mint(),
            results,
        )?));
    }
    println!("  no final answer after {MAX_ROUNDS} rounds");
    Ok(false)
}
