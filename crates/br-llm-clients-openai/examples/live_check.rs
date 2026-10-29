use std::process::ExitCode;

#[path = "live_check/base.rs"]
mod base;
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
    println!("(1) plain answer");
    let plain = verdict("plain answer", checks::plain(&client).await);
    println!("(2) streaming");
    let streaming = verdict("streaming", checks::streaming(&client).await);
    println!("(3) tool call");
    let tool = verdict("tool call", tool_check::weather(&client).await);
    println!("(4) usage");
    let usage = verdict("usage", checks::usage(&client).await);
    println!("(5) unknown model");
    let unknown = verdict("unknown model", checks::unknown_model(&setup).await);
    if plain && streaming && tool && usage && unknown {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
