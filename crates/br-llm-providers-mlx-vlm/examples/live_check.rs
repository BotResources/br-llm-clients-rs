#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::process::ExitCode;

use br_llm_clients_core::{Client, ClientError, OutputMode};
use br_llm_messages::ResponseEnding;
use br_llm_providers_mlx_vlm::Secret;

#[path = "live_check_support/cache.rs"]
mod cache;
#[path = "live_check_support/checks.rs"]
mod checks;
#[path = "live_check_support/env.rs"]
mod env;
#[path = "live_check_support/show.rs"]
mod show;
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
#[path = "live_check_support/weather.rs"]
mod weather;

type Failure = Box<dyn std::error::Error>;

const WRONG_KEY: &str = "mlx-vlm-live-check-wrong-key-0000";
const SHORT_LIMIT: u32 = 8;
const LONG_ANSWER: &str = "Describe the history of Paris in at least ten sentences.";

#[tokio::main]
async fn main() -> ExitCode {
    match live_check().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("live check: {reason}");
            ExitCode::FAILURE
        }
    }
}

async fn live_check() -> Result<(), Failure> {
    let target = target::Target::from_env()?;
    let client = target.client(target.shape())?;
    println!("client: {client:?}");
    let answer = checks::answer(&client).await?;
    println!(
        "[1 plain answer] request id (mlx-vlm sets none on the chat route): {:?}",
        answer.outcome.report().request_id()
    );
    let mut verdicts = answer.verdicts;
    verdicts.push(checks::verdict(
        "4 tool call",
        weather::weather(&client).await?,
    ));
    verdicts.push(checks::verdict(
        "5 structured output",
        checks::structured(&client).await?,
    ));
    if target.shape().key.is_some() {
        let wrong = target.client(target::Shape {
            key: Some(Secret::new(WRONG_KEY)?),
            ..target.shape()
        })?;
        verdicts.push(checks::verdict(
            "6 error case",
            checks::wrong_key(&wrong).await?,
        ));
    } else {
        println!("6 error case: skipped (no MLX_VLM_API_KEY)");
    }
    let short = target.client(target::Shape {
        max_output_tokens: SHORT_LIMIT,
        ..target.shape()
    })?;
    verdicts.push(checks::verdict("7 token limit", token_limit(&short).await?));
    verdicts.push(checks::verdict(
        "8 prefix cache",
        checks::cache_reuse(&client, target::SERVER_OPTIONS).await?,
    ));
    let failed = verdicts.iter().filter(|passed| !**passed).count();
    if failed == 0 {
        println!("live check: every check passed");
        Ok(())
    } else {
        Err(format!("{failed} check(s) failed").into())
    }
}

async fn token_limit(client: &dyn Client) -> Result<bool, Failure> {
    let request = thread::Thread::new(LONG_ANSWER)?.request(None, Vec::new(), OutputMode::Plain)?;
    let (outcome, _) = checks::call(client, "7 token limit", &request).await;
    let ended_on_length = match outcome.result() {
        Ok(response) => matches!(response.ending(), ResponseEnding::OutputExhaustion { .. }),
        Err(ClientError::ReplyUnusable { ending }) => {
            matches!(ending, ResponseEnding::OutputExhaustion { .. })
        }
        Err(_) => false,
    };
    let within = outcome
        .report()
        .usage()
        .and_then(|usage| usage.output_tokens())
        .is_none_or(|tokens| tokens <= u64::from(SHORT_LIMIT));
    Ok(ended_on_length && within)
}
