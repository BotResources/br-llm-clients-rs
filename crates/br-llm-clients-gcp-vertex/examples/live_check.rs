use std::fmt::Display;
use std::process::ExitCode;
use std::time::Duration;

use br_llm_clients_common_http::HttpConfig;
use br_llm_clients_gcp_vertex::HttpClient;

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

const MISSING_MODEL: &str = "claude-does-not-exist";

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
    let vertex = setup.client(&setup.model, &http)?;
    let missing = setup.client(MISSING_MODEL, &http)?;
    println!("== {vertex:?}");
    let verdicts = [
        verdict("plain answer", checks::plain(&vertex).await),
        verdict("streaming", checks::streaming(&vertex).await),
        verdict("tool call", tools::tool_call(&vertex).await),
        verdict("usage", checks::usage(&vertex).await),
        verdict("error case", checks::missing_model(&missing).await),
    ];
    let passed = verdicts.iter().all(|passed| *passed);
    println!("{}", if passed { "ALL PASSED" } else { "SOME FAILED" });
    Ok(passed)
}

fn verdict(name: &str, outcome: Result<(), String>) -> bool {
    match outcome {
        Ok(()) => {
            println!("PASS {name}");
            true
        }
        Err(reason) => {
            println!("FAIL {name}: {reason}");
            false
        }
    }
}
