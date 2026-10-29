use std::env::{self, VarError};
use std::time::Duration;

use br_llm_clients_anthropic::{AnthropicBuildError, AnthropicClient, AnthropicConfig};
use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, Secret};
use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens, ReasoningLevel};
use br_llm_messages::ModelName;

const KEY: &str = "ANTHROPIC_API_KEY";
const BASE_URL: &str = "ANTHROPIC_BASE_URL";
const MODEL: &str = "ANTHROPIC_MODEL";
const WORKSPACE_ID: &str = "ANTHROPIC_WORKSPACE_ID";
const AUTH: &str = "ANTHROPIC_AUTH";
const DEFAULT_MODEL: &str = "claude-haiku-5-5";
const UNKNOWN_MODEL: &str = "claude-no-such-model-0-0";
const MAX_OUTPUT_TOKENS: u32 = 4096;

pub(crate) struct Setup {
    client: AnthropicClient,
    unknown_model_client: AnthropicClient,
    model: ModelName,
    bearer: bool,
    workspace: bool,
}

impl Setup {
    pub(crate) fn from_env() -> Result<Self, String> {
        let key = required(KEY)?;
        let base = required(BASE_URL)?;
        let model = optional(MODEL)?.unwrap_or_else(|| DEFAULT_MODEL.to_owned());
        let workspace_id = optional(WORKSPACE_ID)?;
        let bearer = match optional(AUTH)?.as_deref() {
            None | Some("x-api-key") => false,
            Some("bearer") => true,
            Some(_) => return Err(format!("{AUTH} must be `x-api-key` or `bearer`")),
        };
        let key = Secret::new(key).map_err(|_| format!("{KEY} is not a valid key"))?;
        let base = Endpoint::https(&base)
            .map_err(|_| format!("{BASE_URL} is not a valid full https URL"))?;
        let model =
            ModelName::new(model).map_err(|_| format!("{MODEL} is not a valid model id"))?;
        let unknown_model = ModelName::new(UNKNOWN_MODEL)
            .map_err(|_| "the unknown model id of the check is not valid".to_owned())?;
        let max_output_tokens = MaxOutputTokens::new(MAX_OUTPUT_TOKENS)
            .map_err(|_| "the output limit of the check is not valid".to_owned())?;
        let mut config = AnthropicConfig::new(key).with_base(base);
        let workspace = workspace_id.is_some();
        if let Some(id) = workspace_id {
            config = config.with_workspace_id(id);
        }
        if bearer {
            config = config.with_bearer_auth();
        }
        let http = HttpClient::new(
            HttpConfig::new(
                Duration::from_secs(10),
                Duration::from_secs(60),
                Duration::from_secs(60),
            )
            .with_total_timeout(Duration::from_secs(300)),
        )
        .map_err(|_| "the HTTP client cannot be built".to_owned())?;
        let build = |model: ModelName| {
            let settings = ClientSettings::new(
                model,
                ReasoningLevel::Low,
                CacheRequirement::NotRequired,
                max_output_tokens,
            );
            AnthropicClient::new(settings, config.clone(), http.clone()).map_err(named)
        };
        let client = build(model.clone())?;
        let unknown_model_client = build(unknown_model)?;
        Ok(Self {
            client,
            unknown_model_client,
            model,
            bearer,
            workspace,
        })
    }

    pub(crate) fn describe(&self) -> String {
        format!(
            "model: {}, key header: {}, workspace id: {}",
            self.model.as_str(),
            if self.bearer {
                "authorization (Bearer)"
            } else {
                "x-api-key"
            },
            if self.workspace { "set" } else { "not set" }
        )
    }

    pub(crate) fn client(&self) -> &AnthropicClient {
        &self.client
    }

    pub(crate) fn unknown_model_client(&self) -> &AnthropicClient {
        &self.unknown_model_client
    }
}

fn named(error: AnthropicBuildError) -> String {
    let name = match &error {
        AnthropicBuildError::InvalidCredential { .. } => KEY,
        AnthropicBuildError::InvalidEndpoint { .. } => BASE_URL,
        AnthropicBuildError::InvalidWorkspaceId { .. } => WORKSPACE_ID,
        _ => "the configuration",
    };
    format!("{name} is not valid: {error}")
}

fn required(name: &str) -> Result<String, String> {
    optional(name)?.ok_or_else(|| format!("{name} is not set or is empty"))
}

fn optional(name: &str) -> Result<Option<String>, String> {
    match env::var(name) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(format!("{name} is not valid unicode")),
    }
}
