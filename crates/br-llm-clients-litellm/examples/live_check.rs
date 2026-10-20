use std::env;
use std::process::ExitCode;
use std::time::Duration;

use br_llm_clients_common_http::HttpConfig;
use br_llm_clients_core::{CacheRequirement, Capabilities, ClientSettings, MaxOutputTokens};
use br_llm_clients_litellm::{
    ApiFamily, ChatCompletionsOptions, Endpoint, HttpClient, LiteLlmClient, LiteLlmConfig,
    ReasoningEffort, ReasoningLevel, ReasoningRule, ReasoningTable, Secret,
};
use br_llm_messages::ModelId;

#[path = "live_check_support/env_file.rs"]
mod env_file;
#[path = "live_check_support/scenarios.rs"]
mod scenarios;
#[path = "live_check_support/show.rs"]
mod show;
#[path = "live_check_support/sink.rs"]
mod sink;

type Failure = Box<dyn std::error::Error>;

const DEFAULT_MODEL: &str = "gpt-4o-mini";

fn required(name: &str) -> Result<String, Failure> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("the variable {name} is not set").into())
}

fn base_endpoint(text: &str) -> Result<Endpoint, Failure> {
    let endpoint = if text.starts_with("http://") {
        Endpoint::plain_http(text)
    } else {
        Endpoint::https(text)
    };
    Ok(endpoint?)
}

fn reasoning_table() -> ReasoningTable {
    ReasoningTable::new(
        ReasoningRule::Omit,
        ReasoningRule::Send(ReasoningEffort::Low),
        ReasoningRule::Send(ReasoningEffort::Medium),
        ReasoningRule::Send(ReasoningEffort::High),
        ReasoningRule::Send(ReasoningEffort::High),
        ReasoningRule::Send(ReasoningEffort::High),
    )
}

fn build_client() -> Result<LiteLlmClient, Failure> {
    let key = Secret::new(required("LITELLM_API_KEY")?)?;
    let base = base_endpoint(&required("LITELLM_BASE_URL")?)?;
    let model = env::var("LITELLM_MODEL")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_owned());
    println!("model name: {model}");
    let settings = ClientSettings::new(
        ModelId::new(model)?,
        ReasoningLevel::Off,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(1024)?,
    );
    let family = ApiFamily::ChatCompletions(ChatCompletionsOptions::new(reasoning_table()));
    let config = LiteLlmConfig::new(base, key, family, Capabilities::everything());
    let http = HttpClient::new(
        HttpConfig::new(
            Duration::from_secs(10),
            Duration::from_secs(60),
            Duration::from_secs(60),
        )
        .with_total_timeout(Duration::from_secs(180)),
    )?;
    Ok(LiteLlmClient::new(settings, config, http)?)
}

async fn run() -> Result<bool, Failure> {
    let client = build_client()?;
    println!("client: {client:?}");
    let mut passed = true;
    for (label, result) in [
        ("a: conversation", scenarios::conversation(&client).await),
        ("b: weather tool", scenarios::weather(&client).await),
        ("c: structured output", scenarios::structured(&client).await),
    ] {
        let verdict = result.unwrap_or_else(|error| {
            println!("{label}: error: {error}");
            false
        });
        println!("{label}: {}", if verdict { "ok" } else { "FAILED" });
        passed &= verdict;
    }
    Ok(passed)
}

fn main() -> ExitCode {
    // SAFETY: first statement of main, before Runtime::new, so no other thread exists.
    unsafe { env_file::load() };
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            println!("cannot start the runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(run()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            println!("live check could not start: {error}");
            ExitCode::FAILURE
        }
    }
}
