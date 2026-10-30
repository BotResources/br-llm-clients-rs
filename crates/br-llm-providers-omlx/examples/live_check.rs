#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::process::ExitCode;

use br_llm_clients_core::{CallOutcome, Client, ClientError, OutputMode};
use br_llm_providers_omlx::Secret;

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

const WRONG_KEY: &str = "omlx-live-check-wrong-key-0000";
const UNKNOWN_MODEL: &str = "br-live-check-unknown-model";

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

fn flag(name: &str) -> bool {
    env::optional(name).is_some_and(|value| value == "1")
}

async fn live_check() -> Result<(), Failure> {
    let target = target::Target::from_env()?;
    let client = target.client(target.shape())?;
    println!("client: {client:?}");
    let answer = checks::answer(&client).await?;
    let mut verdicts = answer.verdicts;
    verdicts.push(checks::verdict(
        "4 tool call",
        weather::weather(&client).await?,
    ));
    if flag("OMLX_STRUCTURED") {
        verdicts.push(checks::verdict(
            "5 structured output",
            checks::structured(&client).await?,
        ));
    } else {
        println!(
            "5 structured output: skipped (set OMLX_STRUCTURED=1 for a server with omlx[grammar])"
        );
    }
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
        println!("6 error case: skipped (no OMLX_API_KEY)");
    }
    verdicts.push(checks::verdict("7 request id", request_id(&answer.outcome)));
    verdicts.push(checks::verdict(
        "8 prefix cache",
        checks::cache_reuse(&client, target::SERVER_OPTIONS).await?,
    ));
    if flag("OMLX_MODEL_FALLBACK") {
        println!("9 unknown model: skipped (the model fallback setting of the server is on)");
    } else {
        let unknown = target.client(target::Shape {
            model: UNKNOWN_MODEL.to_owned(),
            ..target.shape()
        })?;
        verdicts.push(checks::verdict(
            "9 unknown model",
            unknown_model(&unknown).await?,
        ));
    }
    let failed = verdicts.iter().filter(|passed| !**passed).count();
    if failed == 0 {
        println!("live check: every check passed");
        Ok(())
    } else {
        Err(format!("{failed} check(s) failed").into())
    }
}

fn request_id(outcome: &CallOutcome) -> bool {
    outcome
        .report()
        .request_id()
        .is_some_and(|id| !id.is_empty())
}

async fn unknown_model(client: &dyn Client) -> Result<bool, Failure> {
    let request =
        thread::Thread::new("Say hello.")?.request(None, Vec::new(), OutputMode::Plain)?;
    let (outcome, _) = checks::call(client, "9 unknown model", &request).await;
    Ok(matches!(outcome.result(), Err(ClientError::ModelNotFound)))
}
