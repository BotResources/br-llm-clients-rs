use br_llm_clients_azure_openai::AzureOpenAiClient;
use br_llm_clients_core::{Client, ClientError};

use crate::Failure;
use crate::setup::{Route, Setup};
use crate::sink::{QuietSink, TextDeltaSink, answer_text};
use crate::thread::Thread;

const SYSTEM: &str = "You answer in one short sentence.";
const QUESTION: &str = "What is the capital of France?";
const UNKNOWN_DEPLOYMENT: &str = "deployment-does-not-exist";

fn show_count(name: &str, count: Option<u64>) {
    match count {
        Some(count) => println!("  {name}: {count}"),
        None => println!("  {name}: not given"),
    }
}

pub async fn plain(client: &AzureOpenAiClient) -> Result<bool, Failure> {
    let request = Thread::new(QUESTION)?.request(SYSTEM, Vec::new())?;
    let outcome = client.call(&request, &mut QuietSink).await;
    match outcome.result() {
        Ok(response) => {
            let text = answer_text(response);
            println!("  answer: {text}");
            Ok(!text.trim().is_empty())
        }
        Err(error) => {
            println!("  error: {error}");
            Ok(false)
        }
    }
}

pub async fn streaming(client: &AzureOpenAiClient) -> Result<bool, Failure> {
    let request = Thread::new("Count from one to ten in words.")?.request(SYSTEM, Vec::new())?;
    let mut sink = TextDeltaSink::default();
    print!("  stream: ");
    let outcome = client.call(&request, &mut sink).await;
    println!();
    println!("  text deltas: {}", sink.text_deltas);
    if let Err(error) = outcome.result() {
        println!("  error: {error}");
        return Ok(false);
    }
    Ok(sink.text_deltas > 0)
}

pub async fn usage(client: &AzureOpenAiClient) -> Result<bool, Failure> {
    let request = Thread::new(QUESTION)?.request(SYSTEM, Vec::new())?;
    let outcome = client.call(&request, &mut QuietSink).await;
    if let Err(error) = outcome.result() {
        println!("  error: {error}");
        return Ok(false);
    }
    let Some(usage) = outcome.report().usage() else {
        println!("  the outcome holds no usage");
        return Ok(false);
    };
    show_count("input tokens", usage.input_tokens());
    show_count("output tokens", usage.output_tokens());
    show_count("cached input tokens", usage.cache_read_input_tokens());
    show_count("reasoning tokens", usage.thinking_tokens());
    Ok(usage.input_tokens().is_some() && usage.output_tokens().is_some())
}

pub async fn unknown_deployment(setup: &Setup, route: Route) -> Result<bool, Failure> {
    let client = setup.client(route, UNKNOWN_DEPLOYMENT)?;
    let request = Thread::new(QUESTION)?.request(SYSTEM, Vec::new())?;
    let outcome = client.call(&request, &mut QuietSink).await;
    match outcome.result() {
        Err(ClientError::ModelNotFound) => {
            println!("  error kind: ModelNotFound");
            Ok(true)
        }
        Err(other) => {
            println!("  expected ModelNotFound, got the error: {other}");
            Ok(false)
        }
        Ok(_) => {
            println!("  expected ModelNotFound, got an answer");
            Ok(false)
        }
    }
}
