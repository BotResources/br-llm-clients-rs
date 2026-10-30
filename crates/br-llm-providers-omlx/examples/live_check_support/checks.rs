use br_llm_clients_core::{
    CallOutcome, Client, ClientError, ClientRequest, OutputMode, Parameter, ParameterKind,
    Parameters, StructuredShape,
};
use br_llm_messages::{ContentBlock, TurnEntry};
use serde_json::{Map, Value};

use crate::Failure;
use crate::cache::Reuse;
use crate::show;
use crate::sink::CountingSink;
use crate::texts::LONG_SYSTEM;
use crate::thread::Thread;
use crate::tools::{results_entry, weather_tool};

const FIRST_QUESTION: &str = "Name one color of the French flag.";
const WEATHER_QUESTION: &str = "What is the weather in Paris?";
const NEXT_QUESTION: &str = "Is it warmer than 10 degrees there?";
const FIELDS: [&str; 3] = ["name", "age", "city"];

pub fn verdict(label: &str, passed: bool) -> bool {
    println!("{label}: {}", if passed { "ok" } else { "FAILED" });
    passed
}

pub async fn call(
    client: &dyn Client,
    label: &str,
    request: &ClientRequest,
) -> (CallOutcome, usize) {
    let mut sink = CountingSink::default();
    let outcome = client.call(request, &mut sink).await;
    println!("[{label}] deltas received: {}", sink.deltas);
    show::outcome(label, &outcome);
    (outcome, sink.deltas)
}

pub struct Answer {
    pub verdicts: Vec<bool>,
    pub outcome: CallOutcome,
}

pub async fn answer(client: &dyn Client) -> Result<Answer, Failure> {
    let request =
        Thread::new(FIRST_QUESTION)?.request(Some(LONG_SYSTEM), Vec::new(), OutputMode::Plain)?;
    let (outcome, deltas) = call(client, "1 plain answer", &request).await;
    let answered = outcome.result().is_ok_and(show::has_text);
    let usage = outcome.report().usage();
    println!("[3 usage] {usage:?}");
    Ok(Answer {
        verdicts: vec![
            verdict("1 plain answer", answered),
            verdict("2 streaming", answered && deltas > 0),
            verdict("3 usage", usage.is_some()),
        ],
        outcome,
    })
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

fn has_fields(block: &ContentBlock) -> bool {
    let ContentBlock::StructuredContent(structured) = block else {
        return false;
    };
    structured
        .value()
        .deserialize_into::<Map<String, Value>>()
        .is_ok_and(|object| FIELDS.iter().all(|field| object.contains_key(*field)))
}

pub async fn structured(client: &dyn Client) -> Result<bool, Failure> {
    let request = Thread::new("Invent a person and give the name, the age and the city.")?
        .request(None, Vec::new(), OutputMode::Structured(person_shape()?))?;
    let (outcome, _) = call(client, "5 structured output", &request).await;
    Ok(outcome
        .result()
        .is_ok_and(|response| response.blocks().iter().any(has_fields)))
}

pub async fn wrong_key(client: &dyn Client) -> Result<bool, Failure> {
    let request = Thread::new("Say hello.")?.request(None, Vec::new(), OutputMode::Plain)?;
    let (outcome, _) = call(client, "6 error case", &request).await;
    Ok(matches!(outcome.result(), Err(ClientError::Authentication)))
}

pub async fn cache_reuse(client: &dyn Client, server_options: &str) -> Result<bool, Failure> {
    let mut thread = Thread::new(WEATHER_QUESTION)?;
    let first = thread.request(Some(LONG_SYSTEM), vec![weather_tool()?], OutputMode::Plain)?;
    let (outcome, _) = call(client, "8 prefix cache, call 1", &first).await;
    let Ok(response) = outcome.result() else {
        return Ok(false);
    };
    let Some((_, results)) = results_entry(response)? else {
        println!("[8 prefix cache] the model called no tool, so the turn has no second call");
        return Ok(false);
    };
    thread.push(TurnEntry::ModelResponse(response.clone()));
    thread.push(results);
    let second = thread.request(Some(LONG_SYSTEM), vec![weather_tool()?], OutputMode::Plain)?;
    let (outcome, _) = call(client, "8 prefix cache, call 2", &second).await;
    let reuse = Reuse::of(&outcome);
    println!("[8 prefix cache] call 2: {}", reuse.describe());
    let reused = outcome.result().is_ok() && reuse.reused();
    if !reused {
        println!("[8 prefix cache] no reuse. {server_options}");
    }
    if let Ok(response) = outcome.result() {
        thread.push(TurnEntry::ModelResponse(response.clone()));
        thread.ask(NEXT_QUESTION)?;
        let third = thread.request(Some(LONG_SYSTEM), vec![weather_tool()?], OutputMode::Plain)?;
        let (outcome, _) = call(client, "8 prefix cache, call 3", &third).await;
        println!(
            "[8 prefix cache] call 3, a new user turn after the turn closed (information only): {}",
            Reuse::of(&outcome).describe()
        );
    }
    Ok(reused)
}
