use std::process::ExitCode;

#[path = "live_check/base.rs"]
mod base;
#[path = "live_check/checks.rs"]
mod checks;
#[path = "live_check/dialogue.rs"]
mod dialogue;
#[path = "live_check/picture_check.rs"]
mod picture_check;
#[path = "live_check/pictures.rs"]
mod pictures;
#[path = "live_check/setup.rs"]
mod setup;
#[path = "live_check/sink.rs"]
mod sink;
#[path = "live_check/thinking_check.rs"]
mod thinking_check;
#[path = "live_check/thread.rs"]
mod thread;
#[path = "live_check/tool_check.rs"]
mod tool_check;

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let setup = match setup::Setup::from_environment() {
        Ok(setup) => setup,
        Err(error) => {
            eprintln!("setup failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let client = match setup.default_client() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("setup failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("model: {}", setup.model());
    let mut passed = Vec::new();
    println!("(1) plain answer");
    passed.push(verdict("plain answer", checks::plain(&client).await));
    println!("(2) streaming");
    passed.push(verdict("streaming", checks::streaming(&client).await));
    println!("(3) tool call");
    passed.push(verdict("tool call", tool_check::weather(&client).await));
    println!("(4) usage");
    passed.push(verdict("usage", checks::usage(&client).await));
    println!("(5) unknown model");
    passed.push(verdict(
        "unknown model",
        checks::unknown_model(&setup).await,
    ));
    println!("(6) one picture");
    passed.push(verdict(
        "one picture",
        picture_check::one_picture(&client).await,
    ));
    println!("(7) two pictures between text blocks");
    passed.push(verdict(
        "two pictures",
        picture_check::two_pictures(&client).await,
    ));
    println!("(8) three-turn dialogue with thinking");
    passed.push(verdict(
        "dialogue with thinking",
        thinking_check::dialogue(&setup).await,
    ));
    if passed.iter().all(|passed| *passed) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
