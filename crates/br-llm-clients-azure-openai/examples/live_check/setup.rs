use std::time::Duration;

use br_llm_clients_azure_openai::{
    AzureOpenAiApi, AzureOpenAiClient, AzureOpenAiConfig, AzureOpenAiVersion,
    ChatCompletionsOptions, ReasoningParameter, ReasoningRule, ReasoningSummary, ReasoningTable,
    ResponsesOptions,
};
use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, Secret};
use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens, ReasoningLevel};
use br_llm_messages::ModelName;

use crate::Failure;

const ENDPOINT_VARIABLE: &str = "AZURE_OPENAI_ENDPOINT";
const KEY_VARIABLE: &str = "AZURE_OPENAI_API_KEY";
const VERSION_VARIABLE: &str = "AZURE_OPENAI_API_VERSION";
const CHAT_DEPLOYMENT_VARIABLE: &str = "AZURE_OPENAI_CHAT_DEPLOYMENT";
pub const RESPONSES_DEPLOYMENT_VARIABLE: &str = "AZURE_OPENAI_RESPONSES_DEPLOYMENT";
const V1_TEXT: &str = "v1";
const MAX_OUTPUT_TOKENS: u32 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    ChatCompletions,
    Responses,
}

impl Route {
    pub fn name(self) -> &'static str {
        match self {
            Route::ChatCompletions => "Chat Completions",
            Route::Responses => "Responses",
        }
    }

    fn api(self) -> AzureOpenAiApi {
        match self {
            Route::ChatCompletions => {
                AzureOpenAiApi::ChatCompletions(ChatCompletionsOptions::new(ReasoningTable::new(
                    ReasoningRule::Omit,
                    ReasoningRule::Refuse,
                    ReasoningRule::Refuse,
                    ReasoningRule::Refuse,
                    ReasoningRule::Refuse,
                    ReasoningRule::Refuse,
                )))
            }
            Route::Responses => {
                let mut options = ResponsesOptions::default();
                options.reasoning_parameter = ReasoningParameter::Omitted;
                options.reasoning_summary = ReasoningSummary::None;
                AzureOpenAiApi::Responses(options)
            }
        }
    }
}

pub struct Setup {
    endpoint: Endpoint,
    key: Secret,
    version: AzureOpenAiVersion,
    chat_deployment: String,
    responses_deployment: Option<String>,
    http: HttpClient,
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn required(name: &str) -> Result<String, Failure> {
    variable(name)
        .ok_or_else(|| format!("the environment variable {name} is missing or empty").into())
}

fn version_of(text: String) -> AzureOpenAiVersion {
    if text.eq_ignore_ascii_case(V1_TEXT) {
        AzureOpenAiVersion::V1
    } else {
        AzureOpenAiVersion::Dated(text)
    }
}

impl Setup {
    pub fn from_environment() -> Result<Self, Failure> {
        let endpoint = Endpoint::https(&required(ENDPOINT_VARIABLE)?)
            .map_err(|error| format!("{ENDPOINT_VARIABLE} is not an https endpoint: {error}"))?;
        let key = Secret::new(required(KEY_VARIABLE)?)?;
        let version = version_of(required(VERSION_VARIABLE)?);
        let chat_deployment = required(CHAT_DEPLOYMENT_VARIABLE)?;
        let responses_deployment = variable(RESPONSES_DEPLOYMENT_VARIABLE);
        let http = HttpClient::new(
            HttpConfig::new(
                Duration::from_secs(10),
                Duration::from_secs(60),
                Duration::from_secs(120),
            )
            .with_total_timeout(Duration::from_secs(300)),
        )?;
        Ok(Self {
            endpoint,
            key,
            version,
            chat_deployment,
            responses_deployment,
            http,
        })
    }

    pub fn version(&self) -> &AzureOpenAiVersion {
        &self.version
    }

    pub fn chat_deployment(&self) -> &str {
        &self.chat_deployment
    }

    pub fn responses_deployment(&self) -> Option<&str> {
        self.responses_deployment.as_deref()
    }

    pub fn client(&self, route: Route, deployment: &str) -> Result<AzureOpenAiClient, Failure> {
        let settings = ClientSettings::new(
            ModelName::new(deployment)?,
            ReasoningLevel::Off,
            CacheRequirement::NotRequired,
            MaxOutputTokens::new(MAX_OUTPUT_TOKENS)?,
        );
        let config = AzureOpenAiConfig::new(
            self.endpoint.clone(),
            self.key.clone(),
            route.api(),
            self.version.clone(),
        );
        Ok(AzureOpenAiClient::new(settings, config, self.http.clone())?)
    }
}
