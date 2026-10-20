use std::process::ExitCode;
use std::time::Duration;

use br_llm_clients_common_http::{HttpClient, HttpConfig, Secret};
use br_llm_clients_core::{
    BoxFuture, CacheRequirement, CallReport, Client, ClientError, ClientRequest, ClientSettings,
    MaxOutputTokens, OutputMode, Parameter, ParameterKind, Parameters, ReasoningLevel, StreamSink,
    StructuredShape, ToolSpec,
};
use br_llm_clients_kimi::{KimiClient, KimiConfig};
use br_llm_messages::{
    AssistantBlock, ModelId, Step, StopReason, StreamEvent, Text, ToolCall, ToolName, ToolResult,
    ToolResultBlock, WireMessage, WireUserBlock,
};
use serde_json::Value;

#[path = "live_check_support/env_file.rs"]
mod env_file;

type Failure = Box<dyn std::error::Error>;

const DEFAULT_MODEL: &str = "kimi-k2.6";
const MAX_ROUNDS: usize = 6;

fn main() -> ExitCode {
    env_file::load();
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("cannot start the runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(run_checks()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("live check stopped: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run_checks() -> Result<bool, Failure> {
    let key = std::env::var("MOONSHOT_API_KEY")
        .map_err(|_| "MOONSHOT_API_KEY is not set in the environment or in a .env file")?;
    let model = std::env::var("MOONSHOT_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());
    let settings = ClientSettings::new(
        ModelId::new(model.clone())?,
        ReasoningLevel::Low,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(8192)?,
    );
    let http = HttpClient::new(
        HttpConfig::new(
            Duration::from_secs(10),
            Duration::from_secs(60),
            Duration::from_secs(120),
        )
        .with_total_timeout(Duration::from_secs(600)),
    )?;
    let client = KimiClient::new(settings, KimiConfig::new(Secret::new(key)?), http)?;
    println!("client: {client:?}");
    let mut all_passed = true;
    for (name, outcome) in [
        ("a. simple conversation", simple(&client).await),
        ("b. two tool calls", tools(&client).await),
        ("c. structured output", structured(&client).await),
    ] {
        match outcome {
            Ok(()) => println!("RESULT {name}: passed\n"),
            Err(reason) => {
                all_passed = false;
                println!("RESULT {name}: FAILED: {reason}\n");
            }
        }
    }
    Ok(all_passed)
}

async fn simple(client: &KimiClient) -> Result<(), String> {
    println!("--- a. simple conversation");
    let request = ClientRequest::new(
        Some(text("You answer in one short sentence.")?),
        vec![user("What is the capital of France?")?],
        Vec::new(),
        OutputMode::Plain,
    )
    .map_err(|error| error.to_string())?;
    let step = call(client, &request).await?;
    expect(step.text().next().is_some(), "the reply holds no text")?;
    expect(
        *step.stop_reason() == StopReason::EndTurn,
        "the finish is not end_turn",
    )
}

async fn tools(client: &KimiClient) -> Result<(), String> {
    println!("--- b. a weather tool called twice");
    let tool = weather_tool()?;
    let mut messages = vec![user(
        "What is the weather in Paris and in Tokyo? Call the tool get_weather once for each city, then summarise in one sentence.",
    )?];
    let mut executed = 0usize;
    for round in 1..=MAX_ROUNDS {
        println!("round {round}");
        let request = ClientRequest::new(
            Some(text("Use the tool to get the weather. Never guess it.")?),
            messages.clone(),
            vec![tool.clone()],
            OutputMode::Plain,
        )
        .map_err(|error| error.to_string())?;
        let step = call(client, &request).await?;
        let calls: Vec<&ToolCall> = step.tool_calls().collect();
        if calls.is_empty() {
            expect(
                executed >= 2,
                &format!("the tool ran {executed} time(s), two were expected"),
            )?;
            return expect(
                step.text().next().is_some(),
                "the final reply holds no text",
            );
        }
        let mut results = Vec::new();
        for call in &calls {
            results.push(WireUserBlock::ToolResult(fake_weather(call)?));
        }
        executed = executed.saturating_add(results.len());
        messages.push(WireMessage::Assistant {
            content: step.content().to_vec(),
        });
        messages.push(WireMessage::User { content: results });
    }
    Err(format!("no final reply after {MAX_ROUNDS} rounds"))
}

async fn structured(client: &KimiClient) -> Result<(), String> {
    println!("--- c. structured output with three fields");
    let shape = StructuredShape::new(
        "city_card",
        Some("A short card about one city".to_owned()),
        Parameters::new(vec![
            field("city", "The name of the city", ParameterKind::String)?,
            field("country", "The country of the city", ParameterKind::String)?,
            field(
                "population_millions",
                "The population of the city in millions",
                ParameterKind::Number,
            )?,
        ])
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let request = ClientRequest::new(
        None,
        vec![user("Give the card of the city of Lyon.")?],
        Vec::new(),
        OutputMode::Structured(shape),
    )
    .map_err(|error| error.to_string())?;
    let step = call(client, &request).await?;
    let value = step
        .structured()
        .next()
        .ok_or("the reply holds no structured value")?;
    for name in ["city", "country", "population_millions"] {
        expect(
            value.get(name).is_some(),
            &format!("the field {name} is missing"),
        )?;
    }
    Ok(())
}

async fn call(client: &KimiClient, request: &ClientRequest) -> Result<Step, String> {
    let mut sink = CountingSink::default();
    let outcome = client.call(request, &mut sink).await;
    print_report(outcome.report(), sink.events);
    let (_, result) = outcome.into_parts();
    let step = result.map_err(|error: ClientError| error.to_string())?;
    print_step(&step);
    Ok(step)
}

fn print_report(report: &CallReport, events: usize) {
    println!("events: {events}");
    println!("model: {:?}", report.model().map(ModelId::as_str));
    println!("request id: {:?}", report.request_id());
    println!("usage: {:?}", report.usage());
    println!(
        "first event after {:?}, total {:?}",
        report.time_to_first_event(),
        report.total_duration()
    );
}

fn print_step(step: &Step) {
    for block in step.content() {
        match block {
            AssistantBlock::Text { text } => println!("text: {}", text.as_str()),
            AssistantBlock::Thinking(thinking) => {
                println!("thinking: {} characters", thinking.text.chars().count());
            }
            AssistantBlock::RedactedThinking(_) => println!("redacted thinking"),
            AssistantBlock::Structured { value } => println!("structured: {value}"),
            AssistantBlock::ToolCall(call) => {
                println!("tool call {}: {}({})", call.id, call.name, call.arguments);
            }
            _ => println!("a block of a kind this example does not know"),
        }
    }
    println!("finish: {}", step.stop_reason());
}

fn weather_tool() -> Result<ToolSpec, String> {
    let name = ToolName::new("get_weather").map_err(|error| error.to_string())?;
    let parameters = Parameters::new(vec![field(
        "city",
        "The name of the city",
        ParameterKind::String,
    )?])
    .map_err(|error| error.to_string())?;
    ToolSpec::new(name, "Gives the current weather of a city", parameters)
        .map_err(|error| error.to_string())
}

fn fake_weather(call: &ToolCall) -> Result<ToolResult, String> {
    let city = call
        .arguments
        .get("city")
        .and_then(Value::as_str)
        .unwrap_or("an unknown city");
    let report = text(&format!("{city}: 18 degrees Celsius, partly cloudy"))?;
    Ok(ToolResult::new(
        call.id.clone(),
        call.name.clone(),
        vec![ToolResultBlock::text(report)],
        false,
    ))
}

fn field(name: &str, description: &str, kind: ParameterKind) -> Result<Parameter, String> {
    Parameter::new(name, description, true, kind).map_err(|error| error.to_string())
}

fn text(value: &str) -> Result<Text, String> {
    Text::new(value).map_err(|error| error.to_string())
}

fn user(value: &str) -> Result<WireMessage, String> {
    Ok(WireMessage::User {
        content: vec![WireUserBlock::Text { text: text(value)? }],
    })
}

fn expect(condition: bool, reason: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(reason.to_owned())
    }
}

#[derive(Default)]
struct CountingSink {
    events: usize,
}

impl StreamSink for CountingSink {
    fn send<'a>(&'a mut self, _event: StreamEvent) -> BoxFuture<'a, ()> {
        self.events = self.events.saturating_add(1);
        Box::pin(async {})
    }
}
