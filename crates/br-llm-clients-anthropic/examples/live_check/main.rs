#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr
)]

macro_rules! say {
    ($($arg:tt)*) => {
        $crate::out::line(format_args!($($arg)*))
    };
}

mod checks;
mod out;
mod report;
mod setup;
mod thread;

use std::process::ExitCode;

use crate::setup::Setup;

#[tokio::main]
async fn main() -> ExitCode {
    let setup = match Setup::from_env() {
        Ok(setup) => setup,
        Err(message) => {
            say!("live_check: {message}");
            return ExitCode::from(2);
        }
    };
    say!("{}", setup.describe());
    let client = setup.client();
    let results = [
        ("plain answer", checks::plain_answer(client).await),
        ("streamed answer", checks::streamed_answer(client).await),
        ("tool round trip", checks::tool_round_trip(client).await),
        ("structured output", checks::structured_output(client).await),
        (
            "unknown model",
            checks::unknown_model(setup.unknown_model_client()).await,
        ),
    ];
    let mut failed = false;
    for (name, result) in results {
        match result {
            Ok(()) => say!("{name}: PASS"),
            Err(error) => {
                failed = true;
                say!("{name}: FAIL {error}");
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
