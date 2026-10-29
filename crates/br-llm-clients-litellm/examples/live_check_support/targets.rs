use std::env;
use std::time::Duration;

use br_llm_clients_common_http::HttpConfig;
use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens};
use br_llm_clients_litellm::{
    ApiFamily, Capabilities, ChatCompletionsOptions, Endpoint, HttpClient, LiteLlmBuildError,
    LiteLlmClient, LiteLlmConfig, ReasoningEffort, ReasoningLevel, ReasoningRule, ReasoningTable,
    Secret,
};
use br_llm_messages::ModelName;

use crate::Failure;
use crate::env::{base_endpoint, key, optional, required};

const DEFAULT_MODEL: &str = "gpt-4o-mini";
const WRONG_KEY: &str = "sk-live-check-wrong-key-0000";
const WRONG_KEY_SOURCE: &str = "the wrong key of the error case";
const OMLX_STRUCTURED: &str =
    "oMLX enforces a structured output only when it is installed with omlx[grammar]";

pub enum Structured {
    Check,
    Skip(&'static str),
}

pub struct Run {
    pub label: &'static str,
    pub model: String,
    pub structured: Structured,
    base: Endpoint,
    key: Secret,
    key_source: &'static str,
    options: ChatCompletionsOptions,
    http: HttpClient,
}

impl Run {
    pub fn client(&self) -> Result<LiteLlmClient, Failure> {
        self.client_for(self.key.clone(), self.key_source)
    }

    pub fn client_with_wrong_key(&self) -> Result<LiteLlmClient, Failure> {
        self.client_for(Secret::new(WRONG_KEY)?, WRONG_KEY_SOURCE)
    }

    fn client_for(&self, key: Secret, key_source: &str) -> Result<LiteLlmClient, Failure> {
        let settings = ClientSettings::new(
            ModelName::new(self.model.as_str())?,
            ReasoningLevel::Off,
            CacheRequirement::NotRequired,
            MaxOutputTokens::new(1024)?,
        );
        let family = ApiFamily::ChatCompletions(self.options.clone());
        let config = LiteLlmConfig::new(self.base.clone(), key, family, Capabilities::everything());
        LiteLlmClient::new(settings, config, self.http.clone()).map_err(|error| match error {
            LiteLlmBuildError::InvalidCredential { .. } => {
                format!("{key_source} is refused: {error}").into()
            }
            other => other.into(),
        })
    }
}

pub fn runs() -> Result<Vec<Run>, Failure> {
    let http = http()?;
    match env::args().nth(1).as_deref() {
        None => cloud(http),
        Some("mlx") => mlx(http),
        Some(other) => Err(format!(
            "unknown target `{other}`: give no argument for the cloud proxy, or `mlx`"
        )
        .into()),
    }
}

fn cloud(http: HttpClient) -> Result<Vec<Run>, Failure> {
    let [api_key, base] = required(["LITELLM_API_KEY", "LITELLM_BASE_URL"])?;
    Ok(vec![Run {
        label: "cloud proxy",
        model: optional("LITELLM_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_owned()),
        structured: Structured::Check,
        base: base_endpoint("LITELLM_BASE_URL", &base)?,
        key: key("LITELLM_API_KEY", api_key)?,
        key_source: "LITELLM_API_KEY",
        options: ChatCompletionsOptions::new(reasoning_table()),
        http,
    }])
}

fn mlx(http: HttpClient) -> Result<Vec<Run>, Failure> {
    let [base, api_key, vlm, omlx] = required([
        "LITELLM_MLX_BASE_URL",
        "LITELLM_MLX_API_KEY",
        "LITELLM_MLX_VLM_MODEL",
        "LITELLM_OMLX_MODEL",
    ])?;
    let base = base_endpoint("LITELLM_MLX_BASE_URL", &base)?;
    let api_key = key("LITELLM_MLX_API_KEY", api_key)?;
    let options =
        ChatCompletionsOptions::new(reasoning_table()).with_reasoning_field("reasoning_content");
    let run = |label, model, structured| Run {
        label,
        model,
        structured,
        base: base.clone(),
        key: api_key.clone(),
        key_source: "LITELLM_MLX_API_KEY",
        options: options.clone(),
        http: http.clone(),
    };
    Ok(vec![
        run("mlx-vlm through the proxy", vlm, Structured::Check),
        run(
            "oMLX through the proxy",
            omlx,
            Structured::Skip(OMLX_STRUCTURED),
        ),
    ])
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

fn http() -> Result<HttpClient, Failure> {
    let config = HttpConfig::new(
        Duration::from_secs(10),
        Duration::from_secs(120),
        Duration::from_secs(60),
    )
    .with_total_timeout(Duration::from_secs(300));
    Ok(HttpClient::new(config)?)
}
