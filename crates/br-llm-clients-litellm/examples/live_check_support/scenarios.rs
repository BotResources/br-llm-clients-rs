use br_llm_clients_core::{
    CallOutcome, Client, ClientRequest, OutputMode, Parameter, ParameterKind, Parameters,
    StructuredShape, ToolSpec,
};
use br_llm_messages::{
    AssistantBlock, Text, ToolName, ToolResult, ToolResultBlock, WireMessage, WireUserBlock,
};

use crate::show;
use crate::sink::CountingSink;

type Failure = Box<dyn std::error::Error>;

const MAX_ROUNDS: usize = 5;

fn user(text: &str) -> Result<WireMessage, Failure> {
    Ok(WireMessage::User {
        content: vec![WireUserBlock::Text {
            text: Text::new(text)?,
        }],
    })
}

async fn call(client: &dyn Client, label: &str, request: &ClientRequest) -> CallOutcome {
    let mut sink = CountingSink::default();
    let outcome = client.call(request, &mut sink).await;
    println!("[{label}] stream events: {}", sink.events);
    show::outcome(label, &outcome);
    outcome
}

pub async fn conversation(client: &dyn Client) -> Result<bool, Failure> {
    let request = ClientRequest::new(
        Some(Text::new("You answer in one short sentence.")?),
        vec![user("Name one color of the French flag.")?],
        Vec::new(),
        OutputMode::Plain,
    )?;
    let outcome = call(client, "a", &request).await;
    Ok(outcome
        .result()
        .is_ok_and(|step| step.text().next().is_some()))
}

fn weather_tool() -> Result<ToolSpec, Failure> {
    let city = Parameter::new("city", "The name of the city", true, ParameterKind::String)?;
    Ok(ToolSpec::new(
        ToolName::new("get_weather")?,
        "Gives the current weather of one city",
        Parameters::new(vec![city])?,
    )?)
}

fn fake_weather(arguments: &serde_json::Value) -> String {
    let city = arguments
        .get("city")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    format!("{city}: 18 degrees, cloudy")
}

pub async fn weather(client: &dyn Client) -> Result<bool, Failure> {
    let system = Text::new("You must call get_weather once for each city, then answer.")?;
    let mut messages = vec![user("What is the weather in Paris and in Tokyo?")?];
    let mut executed = 0usize;
    for round in 0..MAX_ROUNDS {
        let request = ClientRequest::new(
            Some(system.clone()),
            messages.clone(),
            vec![weather_tool()?],
            OutputMode::Plain,
        )?;
        let outcome = call(client, &format!("b{round}"), &request).await;
        let Ok(step) = outcome.result() else {
            return Ok(false);
        };
        let calls: Vec<_> = step
            .content()
            .iter()
            .filter_map(AssistantBlock::as_tool_call)
            .collect();
        if calls.is_empty() {
            println!("[b] tool calls executed: {executed}");
            return Ok(executed >= 2 && step.text().next().is_some());
        }
        let mut results = Vec::new();
        for call in calls {
            executed += 1;
            let text = Text::new(fake_weather(&call.arguments))?;
            results.push(WireUserBlock::ToolResult(ToolResult::new(
                call.id.clone(),
                call.name.clone(),
                vec![ToolResultBlock::text(text)],
                false,
            )));
        }
        messages.push(WireMessage::Assistant {
            content: step.content().to_vec(),
        });
        messages.push(WireMessage::User { content: results });
    }
    println!("[b] no final answer after {MAX_ROUNDS} rounds");
    Ok(false)
}

fn person_shape() -> Result<StructuredShape, Failure> {
    let fields = vec![
        Parameter::new("name", "The first name", true, ParameterKind::String)?,
        Parameter::new("age", "The age in years", true, ParameterKind::Integer)?,
        Parameter::new("city", "The city of residence", true, ParameterKind::String)?,
    ];
    Ok(StructuredShape::new(
        "person",
        Some("A made-up person".to_owned()),
        Parameters::new(fields)?,
    )?)
}

pub async fn structured(client: &dyn Client) -> Result<bool, Failure> {
    let request = ClientRequest::new(
        None,
        vec![user(
            "Invent a person and give the name, the age and the city.",
        )?],
        Vec::new(),
        OutputMode::Structured(person_shape()?),
    )?;
    let outcome = call(client, "c", &request).await;
    let Ok(step) = outcome.result() else {
        return Ok(false);
    };
    Ok(step.content().iter().any(|block| {
        block.as_structured().is_some_and(|value| {
            ["name", "age", "city"]
                .iter()
                .all(|field| value.get(field).is_some())
        })
    }))
}
