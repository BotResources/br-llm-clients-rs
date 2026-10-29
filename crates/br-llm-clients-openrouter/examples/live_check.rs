#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::process::ExitCode;

#[path = "live_check_support/checks.rs"]
mod checks;
#[path = "live_check_support/env.rs"]
mod env;
#[path = "live_check_support/show.rs"]
mod show;
#[path = "live_check_support/sink.rs"]
mod sink;
#[path = "live_check_support/targets.rs"]
mod targets;
#[path = "live_check_support/thread.rs"]
mod thread;
#[path = "live_check_support/weather.rs"]
mod weather;

type Failure = Box<dyn std::error::Error>;

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
    let runs = targets::runs()?;
    let mut failed = 0usize;
    for run in &runs {
        failed = failed.saturating_add(run_checks(run).await?);
    }
    if failed == 0 {
        println!("live check: every check passed");
        Ok(())
    } else {
        Err(format!("{failed} check(s) failed").into())
    }
}

async fn run_checks(run: &targets::Run) -> Result<usize, Failure> {
    println!("== {} (model {})", run.label, run.model);
    let client = run.client()?;
    println!("client: {client:?}");
    let mut verdicts = checks::answer(&client).await?;
    verdicts.push(checks::verdict(
        "4 tool call",
        weather::weather(&client).await?,
    ));
    verdicts.push(checks::verdict(
        "5 structured output",
        checks::structured(&client).await?,
    ));
    let wrong = run.client_with_wrong_key()?;
    verdicts.push(checks::verdict(
        "6 error case",
        checks::wrong_key(&wrong).await?,
    ));
    Ok(verdicts.iter().filter(|passed| !**passed).count())
}
