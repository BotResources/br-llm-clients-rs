use std::time::Duration;

use br_llm_clients_common_http::HttpConfig;
use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens};
use br_llm_messages::ModelName;
use br_llm_providers_omlx::{
    Capabilities, Endpoint, HttpClient, OmlxBuildError, OmlxClient, OmlxConfig, ReasoningLevel,
    ReasoningTable, Secret,
};

use crate::Failure;
use crate::env::{base_endpoint, key, optional, required};

pub const SERVER_OPTIONS: &str = "oMLX reuses its prefix cache only when: the cache is on (no --no-cache); the SSD tier has room (--paged-ssd-cache-dir, --paged-ssd-cache-max-size); the model keeps one name (same alias, same model behind it); the model fallback setting is off.";
const MAX_OUTPUT_TOKENS: u32 = 1024;

pub struct Target {
    base: Endpoint,
    model: String,
    key: Option<Secret>,
    http: HttpClient,
}

pub struct Shape {
    pub model: String,
    pub key: Option<Secret>,
    pub max_output_tokens: u32,
    pub reasoning: ReasoningLevel,
    pub reasoning_carry_back: bool,
}

impl Target {
    pub fn from_env() -> Result<Self, Failure> {
        let [base, model] = required(["OMLX_BASE_URL", "OMLX_MODEL"])?;
        let key = optional("OMLX_API_KEY")
            .map(|text| key("OMLX_API_KEY", text))
            .transpose()?;
        Ok(Self {
            base: base_endpoint("OMLX_BASE_URL", &base)?,
            model,
            key,
            http: http()?,
        })
    }

    pub fn shape(&self) -> Shape {
        Shape {
            model: self.model.clone(),
            key: self.key.clone(),
            max_output_tokens: MAX_OUTPUT_TOKENS,
            reasoning: ReasoningLevel::Off,
            reasoning_carry_back: true,
        }
    }

    pub fn client(&self, shape: Shape) -> Result<OmlxClient, Failure> {
        let settings = ClientSettings::new(
            ModelName::new(shape.model.as_str())?,
            shape.reasoning,
            CacheRequirement::NotRequired,
            MaxOutputTokens::new(shape.max_output_tokens)?,
        );
        let config = OmlxConfig::new(
            self.base.clone(),
            ReasoningTable::openai_effort(),
            Capabilities::everything(),
        )
        .with_reasoning_carry_back(shape.reasoning_carry_back);
        let config = match shape.key {
            Some(key) => config.with_key(key),
            None => config,
        };
        OmlxClient::new(settings, config, self.http.clone()).map_err(|error| match error {
            OmlxBuildError::InvalidCredential { .. } => {
                format!("OMLX_API_KEY is refused: {error}").into()
            }
            other => other.into(),
        })
    }
}

fn http() -> Result<HttpClient, Failure> {
    let config = HttpConfig::new(
        Duration::from_secs(10),
        Duration::from_secs(300),
        Duration::from_secs(120),
    )
    .with_total_timeout(Duration::from_secs(600));
    Ok(HttpClient::new(config)?)
}
