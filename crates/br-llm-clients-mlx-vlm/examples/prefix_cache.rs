#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::process::ExitCode;

use br_llm_clients_core::{Client, OutputMode};
use br_llm_clients_mlx_vlm::ReasoningLevel;
use br_llm_messages::TurnEntry;

#[path = "live_check_support/cache.rs"]
mod cache;
#[path = "live_check_support/env.rs"]
mod env;
#[path = "live_check_support/sink.rs"]
mod sink;
#[path = "live_check_support/target.rs"]
mod target;
#[path = "live_check_support/texts.rs"]
mod texts;
#[path = "live_check_support/thread.rs"]
mod thread;
#[path = "live_check_support/tools.rs"]
mod tools;

type Failure = Box<dyn std::error::Error>;

const CALLS: usize = 4;
const MAX_OUTPUT_TOKENS: u32 = 8192;
const FIRST_QUESTION: &str = "What is the weather in Paris?";
const NEXT_QUESTIONS: [&str; 3] = [
    "Which city did you look up?",
    "Answer again, in fewer words.",
    "Is it warmer than 10 degrees there?",
];

#[tokio::main]
async fn main() -> ExitCode {
    match prefix_cache().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("prefix cache: {reason}");
            ExitCode::FAILURE
        }
    }
}

async fn prefix_cache() -> Result<(), Failure> {
    let target = target::Target::from_env()?;
    let mut reused = false;
    for reasoning_carry_back in [true, false] {
        let client = target.client(target::Shape {
            reasoning: ReasoningLevel::Medium,
            max_output_tokens: MAX_OUTPUT_TOKENS,
            reasoning_carry_back,
            ..target.shape()
        })?;
        let label = if reasoning_carry_back {
            "reasoning sent back"
        } else {
            "reasoning not sent back"
        };
        println!("== {label}: {client:?}");
        reused |= conversation(&client, label).await?;
    }
    if !reused {
        println!(
            "no call read any token from the cache. {}",
            target::SERVER_OPTIONS
        );
    }
    Ok(())
}

async fn conversation(client: &dyn Client, label: &str) -> Result<bool, Failure> {
    let mut thread = thread::Thread::new(FIRST_QUESTION)?;
    let mut questions = NEXT_QUESTIONS.iter();
    let mut reused = false;
    for number in 1..=CALLS {
        let request = thread.request(
            Some(texts::LONG_SYSTEM),
            vec![tools::weather_tool()?],
            OutputMode::Plain,
        )?;
        let mut sink = sink::CountingSink::default();
        let outcome = client.call(&request, &mut sink).await;
        let reuse = cache::Reuse::of(&outcome);
        reused |= reuse.reused();
        println!("[{label}] call {number}: {}", reuse.describe());
        let response = match outcome.result() {
            Ok(response) => response.clone(),
            Err(error) => {
                println!("[{label}] call {number} failed: {error}");
                return Ok(reused);
            }
        };
        let results = tools::results_entry(&response)?;
        thread.push(TurnEntry::ModelResponse(response));
        match (results, questions.next()) {
            (Some((_, results)), _) => thread.push(results),
            (None, Some(question)) => thread.ask(question)?,
            (None, None) => break,
        }
    }
    Ok(reused)
}
