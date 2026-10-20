use br_llm_clients_core::{
    Client, ClientRequest, OutputMode, Parameter, ParameterKind, Parameters, StructuredShape,
    ToolSpec,
};
use br_llm_clients_openai::OpenAiClient;
use br_llm_messages::{
    StopReason, Text, ToolName, ToolResult, ToolResultBlock, WireMessage, WireUserBlock,
};

use crate::describe::{CountingSink, describe};

pub type Failure = Box<dyn std::error::Error>;

const MAX_STEPS: usize = 5;

fn user(text: &str) -> Result<WireMessage, Failure> {
    Ok(WireMessage::User {
        content: vec![WireUserBlock::Text {
            text: Text::new(text)?,
        }],
    })
}

fn required(name: &str, description: &str, kind: ParameterKind) -> Result<Parameter, Failure> {
    Ok(Parameter::new(name, description, true, kind)?)
}

pub async fn simple(client: &OpenAiClient) -> Result<bool, Failure> {
    let request = ClientRequest::new(
        Some(Text::new("You answer in one short sentence.")?),
        vec![user("What is the capital of France?")?],
        Vec::new(),
        OutputMode::Plain,
    )?;
    let mut sink = CountingSink::default();
    let outcome = client.call(&request, &mut sink).await;
    let step = describe(&outcome, sink.events);
    Ok(step.is_some_and(|step| {
        matches!(step.stop_reason(), StopReason::EndTurn) && step.text().next().is_some()
    }))
}

fn weather_tool() -> Result<ToolSpec, Failure> {
    let parameters = Parameters::new(vec![required(
        "city",
        "the name of the city",
        ParameterKind::String,
    )?])?;
    Ok(ToolSpec::new(
        ToolName::new("get_weather")?,
        "Gives the current weather of a city.",
        parameters,
    )?)
}

fn fake_weather(city: &str) -> String {
    format!("{city}: 18 degrees Celsius, partly cloudy")
}

pub async fn weather_tool_twice(client: &OpenAiClient) -> Result<bool, Failure> {
    let system = Text::new("Use the weather tool once for each city before you answer.")?;
    let mut messages = vec![user(
        "What is the weather in Paris and in Tokyo? Give one sentence for each city.",
    )?];
    let tool = weather_tool()?;
    let mut executed = 0usize;
    for round in 1..=MAX_STEPS {
        println!("  round {round}");
        let request = ClientRequest::new(
            Some(system.clone()),
            messages.clone(),
            vec![tool.clone()],
            OutputMode::Plain,
        )?;
        let mut sink = CountingSink::default();
        let outcome = client.call(&request, &mut sink).await;
        let Some(step) = describe(&outcome, sink.events) else {
            return Ok(false);
        };
        messages.push(WireMessage::Assistant {
            content: step.content().to_vec(),
        });
        if !matches!(step.stop_reason(), StopReason::AwaitingToolResults) {
            return Ok(executed >= 2 && matches!(step.stop_reason(), StopReason::EndTurn));
        }
        let mut results = Vec::new();
        for call in step.tool_calls() {
            executed += 1;
            let city = call
                .arguments
                .get("city")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            println!("  fake execution {executed}: {city}");
            results.push(WireUserBlock::ToolResult(ToolResult::new(
                call.id.clone(),
                call.name.clone(),
                vec![ToolResultBlock::text(Text::new(fake_weather(city))?)],
                false,
            )));
        }
        messages.push(WireMessage::User { content: results });
    }
    println!("  no final answer after {MAX_STEPS} rounds");
    Ok(false)
}

pub async fn structured(client: &OpenAiClient) -> Result<bool, Failure> {
    let parameters = Parameters::new(vec![
        required("name", "the name of the city", ParameterKind::String)?,
        required("country", "the country of the city", ParameterKind::String)?,
        required(
            "population_millions",
            "the population in millions of people",
            ParameterKind::Number,
        )?,
    ])?;
    let shape = StructuredShape::new(
        "city_facts",
        Some("Facts about a city".to_owned()),
        parameters,
    )?;
    let request = ClientRequest::new(
        Some(Text::new("You fill the requested structure.")?),
        vec![user("Give the facts about Paris.")?],
        Vec::new(),
        OutputMode::Structured(shape),
    )?;
    let mut sink = CountingSink::default();
    let outcome = client.call(&request, &mut sink).await;
    let step = describe(&outcome, sink.events);
    Ok(step.is_some_and(|step| {
        step.structured()
            .any(|value| value.as_object().is_some_and(|fields| fields.len() == 3))
    }))
}
