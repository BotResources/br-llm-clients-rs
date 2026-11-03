use std::fmt::Display;
use std::process::ExitCode;
use std::time::Duration;

use br_llm_clients_aws_bedrock::HttpClient;
use br_llm_clients_common_http::HttpConfig;

#[path = "live_check_support/checks.rs"]
mod checks;
#[path = "live_check_support/env.rs"]
mod env;
#[path = "live_check_support/report.rs"]
mod report;
#[path = "live_check_support/sink.rs"]
mod sink;
#[path = "live_check_support/thread.rs"]
mod thread;
#[path = "live_check_support/tools.rs"]
mod tools;

const MISSING_MODEL: &str = "eu.anthropic.claude-does-not-exist";

pub(crate) fn fail(error: impl Display) -> String {
    error.to_string()
}

fn main() -> ExitCode {
    let setup = match env::setup() {
        Ok(setup) => setup,
        Err(reason) => {
            eprintln!("live check not started: {reason}");
            return ExitCode::FAILURE;
        }
    };
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
    match runtime.block_on(run_checks(setup)) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(reason) => {
            eprintln!("live check stopped: {reason}");
            ExitCode::FAILURE
        }
    }
}

async fn run_checks(setup: env::Setup) -> Result<bool, String> {
    let http = HttpClient::new(
        HttpConfig::new(
            Duration::from_secs(10),
            Duration::from_secs(120),
            Duration::from_secs(120),
        )
        .with_total_timeout(Duration::from_secs(600)),
    )
    .map_err(fail)?;
    let mut passed = true;
    for path in &setup.paths {
        passed &= run_path(&setup, path, &http).await?;
    }
    println!("{}", if passed { "ALL PASSED" } else { "SOME FAILED" });
    Ok(passed)
}

async fn run_path(setup: &env::Setup, path: &env::Path, http: &HttpClient) -> Result<bool, String> {
    let bedrock = setup.client(path, &setup.model, http)?;
    let missing = setup.client(path, MISSING_MODEL, http)?;
    let label = path.label;
    println!("== {label}: {bedrock:?}");
    let verdicts = [
        verdict(label, "plain answer", checks::plain(&bedrock).await),
        verdict(label, "streaming", checks::streaming(&bedrock).await),
        verdict(label, "tool call", tools::tool_call(&bedrock).await),
        verdict(label, "usage", checks::usage(&bedrock).await),
        verdict(label, "error case", checks::missing_model(&missing).await),
    ];
    Ok(verdicts.iter().all(|passed| *passed))
}

fn verdict(label: &str, name: &str, outcome: Result<(), String>) -> bool {
    match outcome {
        Ok(()) => {
            println!("PASS [{label}] {name}");
            true
        }
        Err(reason) => {
            println!("FAIL [{label}] {name}: {reason}");
            false
        }
    }
}
