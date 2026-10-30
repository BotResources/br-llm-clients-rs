use br_llm_clients_core::{
    CallOutcome, Client, ClientError, ClientRequest, OutputMode, Parameter, ParameterKind,
    Parameters, StructuredShape,
};
use br_llm_clients_litellm::{LiteLlmClient, Map, Value};
use br_llm_messages::ContentBlock;

use crate::Failure;
use crate::show;
use crate::sink::CountingSink;
use crate::thread::Thread;

const SYSTEM: &str = "You answer in one short sentence.";
const FIELDS: [&str; 3] = ["name", "age", "city"];

pub fn verdict(label: &str, passed: bool) -> bool {
    println!("{label}: {}", if passed { "ok" } else { "FAILED" });
    passed
}

pub async fn call(
    client: &LiteLlmClient,
    label: &str,
    request: &ClientRequest,
) -> (CallOutcome, usize) {
    let mut sink = CountingSink::default();
    let outcome = client.call(request, &mut sink).await;
    println!("[{label}] deltas received: {}", sink.deltas);
    show::outcome(label, &outcome);
    (outcome, sink.deltas)
}

pub async fn answer(client: &LiteLlmClient) -> Result<Vec<bool>, Failure> {
    let request = Thread::new("Name one color of the French flag.")?.request(
        Some(SYSTEM),
        Vec::new(),
        OutputMode::Plain,
    )?;
    let (outcome, deltas) = call(client, "1 plain answer", &request).await;
    let answered = outcome.result().is_ok_and(show::has_text);
    let usage = outcome.report().usage();
    println!("[3 usage] {usage:?}");
    Ok(vec![
        verdict("1 plain answer", answered),
        verdict("2 streaming", answered && deltas > 0),
        verdict("3 usage", usage.is_some()),
    ])
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

pub async fn structured(client: &LiteLlmClient) -> Result<bool, Failure> {
    let request = Thread::new("Invent a person and give the name, the age and the city.")?
        .request(None, Vec::new(), OutputMode::Structured(person_shape()?))?;
    let (outcome, _) = call(client, "5 structured output", &request).await;
    Ok(outcome
        .result()
        .is_ok_and(|response| response.blocks().iter().any(has_fields)))
}

pub async fn wrong_key(client: &LiteLlmClient) -> Result<bool, Failure> {
    let request = Thread::new("Say hello.")?.request(None, Vec::new(), OutputMode::Plain)?;
    let (outcome, _) = call(client, "6 error case", &request).await;
    Ok(matches!(outcome.result(), Err(ClientError::Authentication)))
}
