use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use br_llm_clients_anthropic::{AnthropicClient, AnthropicConfig};
use br_llm_clients_common_http::{HttpClient, HttpConfig, Secret};
use br_llm_clients_core::{
    BoxFuture, CacheRequirement, Client, ClientRequest, ClientSettings, MaxOutputTokens,
    OutputMode, Parameter, ParameterKind, Parameters, ReasoningLevel, StreamSink, StructuredShape,
    ToolSpec,
};
use br_llm_messages::{
    AssistantBlock, ModelId, Step, StreamEvent, Text, ToolResult, ToolResultBlock, WireMessage,
    WireUserBlock,
};

type Failure = Box<dyn Error>;

const KEY_VARIABLE: &str = "ANTHROPIC_API_KEY";
const MODEL_VARIABLE: &str = "ANTHROPIC_MODEL";
const DEFAULT_MODEL: &str = "claude-haiku-5-5";
const MAX_ROUNDS: usize = 6;

struct CountingSink {
    events: usize,
}

impl StreamSink for CountingSink {
    fn send<'a>(&'a mut self, _event: StreamEvent) -> BoxFuture<'a, ()> {
        self.events += 1;
        Box::pin(async {})
    }
}

fn parse_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (name, value) = line.split_once('=')?;
    let value = value.trim().trim_matches('"').trim_matches('\'');
    (!value.is_empty()).then(|| (name.trim().to_owned(), value.to_owned()))
}

fn load_env_files() -> HashMap<String, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    let mut loaded = HashMap::new();
    for path in [PathBuf::from(".env"), root] {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        for (name, value) in content.lines().filter_map(parse_line) {
            loaded.entry(name).or_insert(value);
        }
    }
    loaded
}

fn lookup(file: &HashMap<String, String>, name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| file.get(name).cloned())
}

fn user(blocks: Vec<WireUserBlock>) -> WireMessage {
    WireMessage::User { content: blocks }
}

fn user_text(text: &str) -> Result<WireMessage, Failure> {
    Ok(user(vec![WireUserBlock::Text {
        text: Text::new(text)?,
    }]))
}

fn required(name: &str, text: &str, kind: ParameterKind) -> Result<Parameter, Failure> {
    Ok(Parameter::new(name, text, true, kind)?)
}

fn weather_tool() -> Result<ToolSpec, Failure> {
    let city = required("city", "the name of the city", ParameterKind::String)?;
    Ok(ToolSpec::new(
        br_llm_messages::ToolName::new("get_weather")?,
        "Gives the current weather of one city",
        Parameters::new(vec![city])?,
    )?)
}

fn city_shape() -> Result<StructuredShape, Failure> {
    let parameters = Parameters::new(vec![
        required("city", "the name of the city", ParameterKind::String)?,
        required("country", "the country of the city", ParameterKind::String)?,
        required("population_millions", "in millions", ParameterKind::Number)?,
    ])?;
    Ok(StructuredShape::new(
        "city_card",
        Some("A short card about one city".to_owned()),
        parameters,
    )?)
}

fn describe(step: &Step) {
    for block in step.content() {
        match block {
            AssistantBlock::Text { text } => println!("  text: {text}"),
            AssistantBlock::Thinking(thinking) => {
                println!("  thinking: {} characters", thinking.text.chars().count());
            }
            AssistantBlock::ToolCall(call) => {
                println!("  tool call: {} {} {}", call.id, call.name, call.arguments);
            }
            AssistantBlock::Structured { value } => println!("  structured: {value}"),
            _ => println!("  another kind of block"),
        }
    }
    println!("  finish: {}", step.stop_reason());
    println!("  usage: {:?}", step.usage());
}

async fn call(client: &AnthropicClient, request: &ClientRequest) -> Result<Step, Failure> {
    let mut sink = CountingSink { events: 0 };
    let outcome = client.call(request, &mut sink).await;
    let (report, result) = outcome.into_parts();
    println!(
        "  events: {}, request id: {:?}, duration: {:?}, first event: {:?}",
        sink.events,
        report.request_id(),
        report.total_duration(),
        report.time_to_first_event()
    );
    let step = result?;
    describe(&step);
    Ok(step)
}

fn request(
    system: &str,
    messages: Vec<WireMessage>,
    tools: Vec<ToolSpec>,
    output: OutputMode,
) -> Result<ClientRequest, Failure> {
    Ok(ClientRequest::new(
        Some(Text::new(system)?),
        messages,
        tools,
        output,
    )?)
}

async fn simple_conversation(client: &AnthropicClient) -> Result<(), Failure> {
    let asked = request(
        "You answer in one short sentence.",
        vec![user_text("What is the capital of France?")?],
        Vec::new(),
        OutputMode::Plain,
    )?;
    let step = call(client, &asked).await?;
    if step.text().next().is_none() {
        return Err("the reply holds no text".into());
    }
    Ok(())
}

fn fake_weather(call: &br_llm_messages::ToolCall) -> Result<WireUserBlock, Failure> {
    let city = call
        .arguments
        .get("city")
        .and_then(|value| value.as_str())
        .unwrap_or("an unknown city");
    let text = Text::new(format!("{city}: 18 degrees Celsius, clear sky"))?;
    Ok(WireUserBlock::ToolResult(ToolResult::new(
        call.id.clone(),
        call.name.clone(),
        vec![ToolResultBlock::text(text)],
        false,
    )))
}

async fn tool_conversation(client: &AnthropicClient) -> Result<(), Failure> {
    let system =
        "You are a weather assistant. Always use the tool to read a weather, one call per city.";
    let mut messages = vec![user_text(
        "What is the weather in Paris and in Tokyo? Use the tool for each city.",
    )?];
    let mut executed = 0usize;
    for round in 1..=MAX_ROUNDS {
        println!("  round {round}");
        let asked = request(
            system,
            messages.clone(),
            vec![weather_tool()?],
            OutputMode::Plain,
        )?;
        let step = call(client, &asked).await?;
        let calls: Vec<_> = step
            .content()
            .iter()
            .filter_map(AssistantBlock::as_tool_call)
            .collect();
        if calls.is_empty() {
            if executed < 2 {
                return Err(format!("the tool was called {executed} times, two expected").into());
            }
            println!("  tool executions: {executed}");
            return Ok(());
        }
        let results = calls
            .iter()
            .map(|call| fake_weather(call))
            .collect::<Result<Vec<_>, _>>()?;
        executed += results.len();
        messages.push(WireMessage::Assistant {
            content: step.content().to_vec(),
        });
        messages.push(user(results));
    }
    Err("the model kept calling the tool".into())
}

async fn structured_reply(client: &AnthropicClient) -> Result<(), Failure> {
    let asked = request(
        "You fill the card that is asked for.",
        vec![user_text("Give the card of the city of Lyon.")?],
        Vec::new(),
        OutputMode::Structured(city_shape()?),
    )?;
    let step = call(client, &asked).await?;
    let value = step
        .content()
        .iter()
        .find_map(AssistantBlock::as_structured)
        .ok_or("the reply holds no structured value")?;
    for field in ["city", "country", "population_millions"] {
        if value.get(field).is_none() {
            return Err(format!("the field {field} is missing").into());
        }
    }
    Ok(())
}

fn build_client(file: &HashMap<String, String>) -> Result<AnthropicClient, Failure> {
    let key = lookup(file, KEY_VARIABLE).ok_or(format!("{KEY_VARIABLE} is not set"))?;
    let model = lookup(file, MODEL_VARIABLE).unwrap_or_else(|| DEFAULT_MODEL.to_owned());
    println!("model: {model}");
    let settings = ClientSettings::new(
        ModelId::new(model)?,
        ReasoningLevel::Low,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(4096)?,
    );
    let http = HttpClient::new(
        HttpConfig::new(
            Duration::from_secs(10),
            Duration::from_secs(60),
            Duration::from_secs(60),
        )
        .with_total_timeout(Duration::from_secs(300)),
    )?;
    Ok(AnthropicClient::new(
        settings,
        AnthropicConfig::new(Secret::new(key)?),
        http,
    )?)
}

#[tokio::main]
async fn main() -> ExitCode {
    let client = match build_client(&load_env_files()) {
        Ok(client) => client,
        Err(error) => {
            println!("setup failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut failed = false;
    for (name, outcome) in [
        ("simple conversation", simple_conversation(&client).await),
        ("tool conversation", tool_conversation(&client).await),
        ("structured reply", structured_reply(&client).await),
    ] {
        match outcome {
            Ok(()) => println!("{name}: PASS"),
            Err(error) => {
                failed = true;
                println!("{name}: FAIL {error}");
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
