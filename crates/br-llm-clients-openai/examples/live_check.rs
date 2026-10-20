use std::process::ExitCode;
use std::time::Duration;

use br_llm_clients_common_http::{HttpClient, HttpConfig, Secret};
use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens, ReasoningLevel};
use br_llm_clients_openai::{ModelProfile, OpenAiClient, OpenAiConfig};
use br_llm_messages::ModelId;

#[path = "live_check/describe.rs"]
mod describe;
#[path = "live_check/dotenv.rs"]
mod dotenv;
#[path = "live_check/scenarios.rs"]
mod scenarios;

use scenarios::Failure;

const DEFAULT_MODEL: &str = "gpt-5.4-mini";
const KEY_VARIABLE: &str = "OPENAI_API_KEY";
const MODEL_VARIABLE: &str = "OPENAI_MODEL";

fn build_client(settings: &dotenv::Settings) -> Result<(OpenAiClient, String), Failure> {
    let key = settings
        .get(KEY_VARIABLE)
        .ok_or("OPENAI_API_KEY is not set in the environment or in a .env file")?;
    let model = settings
        .get(MODEL_VARIABLE)
        .unwrap_or_else(|| DEFAULT_MODEL.to_owned());
    let client_settings = ClientSettings::new(
        ModelId::new(model.clone())?,
        ReasoningLevel::Low,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(4096)?,
    );
    let http = HttpClient::new(
        HttpConfig::new(
            Duration::from_secs(10),
            Duration::from_secs(60),
            Duration::from_secs(120),
        )
        .with_total_timeout(Duration::from_secs(300)),
    )?;
    let config = OpenAiConfig::new(Secret::new(key)?, ModelProfile::gpt_5_4_mini());
    Ok((OpenAiClient::new(client_settings, config, http)?, model))
}

fn verdict(name: &str, result: Result<bool, Failure>) -> bool {
    match result {
        Ok(true) => {
            println!("RESULT {name}: pass");
            true
        }
        Ok(false) => {
            println!("RESULT {name}: fail");
            false
        }
        Err(error) => {
            println!("RESULT {name}: could not run: {error}");
            false
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let settings = dotenv::Settings::load();
    let (client, model) = match build_client(&settings) {
        Ok(built) => built,
        Err(error) => {
            println!("setup failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("model: {model}");
    if model != DEFAULT_MODEL {
        println!("note: the preset gpt_5_4_mini still applies to the model {model}");
    }
    println!("(a) simple conversation");
    let a = verdict("a", scenarios::simple(&client).await);
    println!("(b) weather tool called twice");
    let b = verdict("b", scenarios::weather_tool_twice(&client).await);
    println!("(c) structured output with three fields");
    let c = verdict("c", scenarios::structured(&client).await);
    if a && b && c {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
