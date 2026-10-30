use std::collections::BTreeMap;

use br_llm_clients_anthropic::AnthropicClient;
use br_llm_clients_core::{
    Client, ClientError, OutputMode, Parameter, ParameterKind, Parameters, StructuredShape,
    ToolSpec,
};
use br_llm_messages::{ContentBlock, ModelResponse, OpaqueJson, ToolName};

use crate::report::{PrintingSink, QuietSink, answered, error_kind, print_report};
use crate::thread::{Failure, Thread, fake_weather};

const SHORT: &str = "You answer in one short sentence.";
const MAX_ROUNDS: usize = 4;
const CARD_FIELDS: [&str; 3] = ["city", "country", "population_millions"];

pub(crate) async fn plain_answer(client: &AnthropicClient) -> Result<(), Failure> {
    let thread = Thread::new("What is the capital of France?")?;
    let request = thread.request(SHORT, Vec::new(), OutputMode::Plain)?;
    let mut sink = QuietSink::default();
    let response = answered(client.call(&request, &mut sink).await)?;
    say!("  deltas received: {}", sink.deltas);
    require_text(&response)
}

pub(crate) async fn streamed_answer(client: &AnthropicClient) -> Result<(), Failure> {
    let thread = Thread::new("Name the four seasons of the year, in one sentence.")?;
    let request = thread.request(SHORT, Vec::new(), OutputMode::Plain)?;
    let mut sink = PrintingSink::default();
    say!("  streamed:");
    let outcome = client.call(&request, &mut sink).await;
    say!("");
    let response = answered(outcome)?;
    say!("  text parts streamed: {}", sink.text_parts);
    if sink.text_parts == 0 {
        return Err("no text part was streamed".into());
    }
    require_text(&response)
}

pub(crate) async fn tool_round_trip(client: &AnthropicClient) -> Result<(), Failure> {
    let system =
        "You are a weather assistant. Always read the weather with the tool before you answer.";
    let mut thread = Thread::new("What is the weather in Paris right now? Use the tool.")?;
    let mut executed = 0usize;
    for round in 1..=MAX_ROUNDS {
        say!("  round {round}");
        let request = thread.request(system, vec![weather_tool()?], OutputMode::Plain)?;
        let mut sink = QuietSink::default();
        let response = answered(client.call(&request, &mut sink).await)?;
        let results = response
            .runtime_calls()
            .map(fake_weather)
            .collect::<Result<Vec<_>, _>>()?;
        if results.is_empty() {
            if executed == 0 {
                return Err("the model answered without calling the tool".into());
            }
            say!("  tool executions: {executed}");
            return require_text(&response);
        }
        executed = executed.saturating_add(results.len());
        thread.answer_tools(response, results)?;
    }
    Err(format!("the model still called the tool after {MAX_ROUNDS} rounds").into())
}

pub(crate) async fn structured_output(client: &AnthropicClient) -> Result<(), Failure> {
    let thread = Thread::new("Fill the card of the city of Lyon.")?;
    let request = thread.request(
        "You fill the card that is asked for.",
        Vec::new(),
        OutputMode::Structured(city_card()?),
    )?;
    let mut sink = QuietSink::default();
    let response = answered(client.call(&request, &mut sink).await)?;
    let value = response
        .blocks()
        .iter()
        .find_map(|block| match block {
            ContentBlock::StructuredContent(structured) => Some(structured.value()),
            _ => None,
        })
        .ok_or("the reply holds no structured value")?;
    let fields: BTreeMap<String, OpaqueJson> = value.deserialize_into()?;
    for field in CARD_FIELDS {
        if !fields.contains_key(field) {
            return Err(format!("the field {field} is missing").into());
        }
    }
    Ok(())
}

pub(crate) async fn unknown_model(client: &AnthropicClient) -> Result<(), Failure> {
    let thread = Thread::new("Say hello.")?;
    let request = thread.request(SHORT, Vec::new(), OutputMode::Plain)?;
    let mut sink = QuietSink::default();
    let (report, result) = client.call(&request, &mut sink).await.into_parts();
    print_report(&report);
    let error = match result {
        Ok(_) => return Err("an unknown model id gave an answer".into()),
        Err(error) => error,
    };
    let kind = error_kind(&error);
    if is_model_refusal(&error) {
        say!("  refused: {kind}");
        Ok(())
    } else {
        Err(format!("the unknown model id gave {kind}, not a refusal of the model").into())
    }
}

fn is_model_refusal(error: &ClientError) -> bool {
    match error {
        ClientError::ModelNotFound | ClientError::ModelNotAccessible => true,
        ClientError::ProviderRejected {
            status: Some(404),
            code: Some(code),
            ..
        } => code == "not_found_error",
        _ => false,
    }
}

fn require_text(response: &ModelResponse) -> Result<(), Failure> {
    let has_text = response
        .blocks()
        .iter()
        .any(|block| matches!(block, ContentBlock::Text(_)));
    if has_text {
        Ok(())
    } else {
        Err("the reply holds no text".into())
    }
}

fn required(name: &str, description: &str, kind: ParameterKind) -> Result<Parameter, Failure> {
    Ok(Parameter::new(name, description, true, kind)?)
}

fn weather_tool() -> Result<ToolSpec, Failure> {
    let city = required("city", "the name of the city", ParameterKind::String)?;
    Ok(ToolSpec::new(
        ToolName::new("get_weather")?,
        "Gives the current weather of one city",
        Parameters::new(vec![city])?,
    )?)
}

fn city_card() -> Result<StructuredShape, Failure> {
    let parameters = Parameters::new(vec![
        required("city", "the name of the city", ParameterKind::String)?,
        required("country", "the country of the city", ParameterKind::String)?,
        required(
            "population_millions",
            "the population, in millions",
            ParameterKind::Number,
        )?,
    ])?;
    Ok(StructuredShape::new(
        "city_card",
        Some("A short card about one city".to_owned()),
        parameters,
    )?)
}
