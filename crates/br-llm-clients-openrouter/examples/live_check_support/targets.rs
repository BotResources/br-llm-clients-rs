use std::env;
use std::time::Duration;

use br_llm_clients_common_http::HttpConfig;
use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens};
use br_llm_clients_openrouter::{
    ApiFamily, Capabilities, ChatCompletionsOptions, Endpoint, HttpClient, MessagesOptions,
    OpenRouterBuildError, OpenRouterClient, OpenRouterConfig, Options, ReasoningEffort,
    ReasoningLevel, ReasoningRule, ReasoningTable, Secret,
};
use br_llm_messages::ModelName;

use crate::Failure;
use crate::env::{base_endpoint, key, optional, required};

const DEFAULT_MODEL: &str = "openai/gpt-4o-mini";
const REASONING_FIELD: &str = "reasoning";
const WRONG_KEY: &str = "sk-or-v1-live-check-wrong-key-0000";
const WRONG_KEY_SOURCE: &str = "the wrong key of the error case";

pub struct Run {
    pub label: &'static str,
    pub model: String,
    family: ApiFamily,
    base: Option<Endpoint>,
    app_referer: Option<String>,
    app_title: Option<String>,
    key: Secret,
    http: HttpClient,
}

impl Run {
    pub fn client(&self) -> Result<OpenRouterClient, Failure> {
        self.client_for(self.key.clone(), "OPENROUTER_API_KEY")
    }

    pub fn client_with_wrong_key(&self) -> Result<OpenRouterClient, Failure> {
        self.client_for(Secret::new(WRONG_KEY)?, WRONG_KEY_SOURCE)
    }

    fn client_for(&self, key: Secret, key_source: &str) -> Result<OpenRouterClient, Failure> {
        let settings = ClientSettings::new(
            ModelName::new(self.model.as_str())?,
            ReasoningLevel::Off,
            CacheRequirement::NotRequired,
            MaxOutputTokens::new(1024)?,
        );
        let mut config =
            OpenRouterConfig::new(key, self.family.clone(), Capabilities::everything());
        if let Some(base) = &self.base {
            config = config.with_base(base.clone());
        }
        if let Some(referer) = &self.app_referer {
            config = config.with_app_referer(referer.as_str());
        }
        if let Some(title) = &self.app_title {
            config = config.with_app_title(title.as_str());
        }
        OpenRouterClient::new(settings, config, self.http.clone()).map_err(|error| match error {
            OpenRouterBuildError::InvalidCredential { .. } => {
                format!("{key_source} is refused: {error}").into()
            }
            other => other.into(),
        })
    }
}

pub fn runs() -> Result<Vec<Run>, Failure> {
    let (label, family) = match env::args().nth(1).as_deref() {
        None | Some("chat") => ("chat completions route", chat_family()),
        Some("responses") => ("responses route", ApiFamily::Responses(Options::default())),
        Some("messages") => (
            "messages route",
            ApiFamily::Messages(MessagesOptions::new()),
        ),
        Some(other) => {
            return Err(format!(
                "unknown route `{other}`: give no argument or `chat`, `responses` or `messages`"
            )
            .into());
        }
    };
    let base = optional("OPENROUTER_BASE_URL")
        .map(|text| base_endpoint("OPENROUTER_BASE_URL", &text))
        .transpose()?;
    Ok(vec![Run {
        label,
        model: optional("OPENROUTER_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_owned()),
        family,
        base,
        app_referer: optional("OPENROUTER_APP_REFERER"),
        app_title: optional("OPENROUTER_APP_TITLE"),
        key: key("OPENROUTER_API_KEY", required("OPENROUTER_API_KEY")?)?,
        http: http()?,
    }])
}

fn chat_family() -> ApiFamily {
    ApiFamily::ChatCompletions(
        ChatCompletionsOptions::new(reasoning_table())
            .with_reasoning_field(REASONING_FIELD)
            .with_reasoning_carry_back_field(REASONING_FIELD),
    )
}

fn reasoning_table() -> ReasoningTable {
    ReasoningTable::new(
        ReasoningRule::Omit,
        ReasoningRule::Send(ReasoningEffort::Low),
        ReasoningRule::Send(ReasoningEffort::Medium),
        ReasoningRule::Send(ReasoningEffort::High),
        ReasoningRule::Send(ReasoningEffort::Xhigh),
        ReasoningRule::Send(ReasoningEffort::Max),
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
