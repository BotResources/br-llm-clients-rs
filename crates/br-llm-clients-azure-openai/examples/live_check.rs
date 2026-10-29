use std::process::ExitCode;

#[path = "live_check/checks.rs"]
mod checks;
#[path = "live_check/setup.rs"]
mod setup;
#[path = "live_check/sink.rs"]
mod sink;
#[path = "live_check/thread.rs"]
mod thread;
#[path = "live_check/tool_check.rs"]
mod tool_check;

use setup::{RESPONSES_DEPLOYMENT_VARIABLE, Route, Setup};

pub type Failure = Box<dyn std::error::Error>;

fn verdict(name: &str, result: Result<bool, Failure>) -> bool {
    match result {
        Ok(true) => {
            println!("PASS {name}");
            true
        }
        Ok(false) => {
            println!("FAIL {name}");
            false
        }
        Err(error) => {
            println!("FAIL {name}: could not run: {error}");
            false
        }
    }
}

async fn check_route(setup: &Setup, route: Route, deployment: &str) -> bool {
    println!("== {} route, deployment {deployment}", route.name());
    let client = match setup.client(route, deployment) {
        Ok(client) => client,
        Err(error) => {
            println!("FAIL {} client: could not build: {error}", route.name());
            return false;
        }
    };
    println!("(1) plain answer");
    let plain = verdict("plain answer", checks::plain(&client).await);
    println!("(2) streaming");
    let streaming = verdict("streaming", checks::streaming(&client).await);
    println!("(3) tool call");
    let tool = verdict("tool call", tool_check::weather(&client).await);
    println!("(4) usage");
    let usage = verdict("usage", checks::usage(&client).await);
    println!("(5) unknown deployment");
    let unknown = verdict(
        "unknown deployment",
        checks::unknown_deployment(setup, route).await,
    );
    plain && streaming && tool && usage && unknown
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let setup = match Setup::from_environment() {
        Ok(setup) => setup,
        Err(error) => {
            eprintln!("setup failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("api version: {:?}", setup.version());
    let mut passed = check_route(&setup, Route::ChatCompletions, setup.chat_deployment()).await;
    if let Some(deployment) = setup.responses_deployment() {
        passed &= check_route(&setup, Route::Responses, deployment).await;
    } else {
        println!("== Responses route skipped: {RESPONSES_DEPLOYMENT_VARIABLE} is not set");
    }
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
