use std::time::Duration;

use br_llm_clients_common_http::HttpConfig;
use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens};
use br_llm_clients_mlx_vlm::{
    Capabilities, Endpoint, HttpClient, MlxVlmBuildError, MlxVlmClient, MlxVlmConfig,
    ReasoningLevel, ReasoningTable, Secret,
};
use br_llm_messages::ModelName;

use crate::Failure;
use crate::env::{base_endpoint, key, optional, required};

pub const SERVER_OPTIONS: &str = "mlx-vlm reuses its prefix cache only when: APC_ENABLED=1 (or apc_enabled through PATCH /v1/settings); APC_NUM_BLOCKS is large enough for the prompts; APC_HASH=sha256 when the reuse must survive a restart; the server serves one model; the calls share one tenant (the same X-APC-Tenant header, or none).";
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
        let [base, model] = required(["MLX_VLM_BASE_URL", "MLX_VLM_MODEL"])?;
        let key = optional("MLX_VLM_API_KEY")
            .map(|text| key("MLX_VLM_API_KEY", text))
            .transpose()?;
        Ok(Self {
            base: base_endpoint("MLX_VLM_BASE_URL", &base)?,
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

    pub fn client(&self, shape: Shape) -> Result<MlxVlmClient, Failure> {
        let settings = ClientSettings::new(
            ModelName::new(shape.model.as_str())?,
            shape.reasoning,
            CacheRequirement::NotRequired,
            MaxOutputTokens::new(shape.max_output_tokens)?,
        );
        let config = MlxVlmConfig::new(
            self.base.clone(),
            ReasoningTable::openai_effort(),
            Capabilities::everything(),
        )
        .with_reasoning_carry_back(shape.reasoning_carry_back);
        let config = match shape.key {
            Some(key) => config.with_key(key),
            None => config,
        };
        MlxVlmClient::new(settings, config, self.http.clone()).map_err(|error| match error {
            MlxVlmBuildError::InvalidCredential { .. } => {
                format!("MLX_VLM_API_KEY is refused: {error}").into()
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
