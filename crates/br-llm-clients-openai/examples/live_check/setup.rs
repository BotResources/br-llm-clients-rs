use std::time::Duration;

use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig, Secret};
use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens, ReasoningLevel};
use br_llm_clients_openai::{Accepted, ModelProfile, OpenAiClient, OpenAiConfig};
use br_llm_messages::ModelName;

use crate::Failure;
use crate::base::{BASE_VARIABLE, base_endpoint};

const KEY_VARIABLE: &str = "OPENAI_API_KEY";
const MODEL_VARIABLE: &str = "OPENAI_MODEL";
const DEFAULT_MODEL: &str = "gpt-5.4-mini";
const MAX_OUTPUT_TOKENS: u32 = 4096;

pub struct Setup {
    key: Secret,
    base: Option<Endpoint>,
    model: String,
    profile: ModelProfile,
    http: HttpClient,
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn required(name: &str) -> Result<String, Failure> {
    variable(name)
        .ok_or_else(|| format!("the environment variable {name} is missing or empty").into())
}

fn preset_of(model: &str) -> Option<ModelProfile> {
    match model {
        "gpt-6-astra" => Some(ModelProfile::gpt_6_astra()),
        "gpt-6.1-sol" => Some(ModelProfile::gpt_6_1_sol()),
        "gpt-6.1-terra" => Some(ModelProfile::gpt_6_1_terra()),
        "gpt-6-sol" => Some(ModelProfile::gpt_6_sol()),
        "gpt-6-luna" => Some(ModelProfile::gpt_6_luna()),
        "gpt-5.6-sol" | "gpt-5.6" => Some(ModelProfile::gpt_5_6_sol()),
        "gpt-5.5" => Some(ModelProfile::gpt_5_5()),
        "gpt-5.4" => Some(ModelProfile::gpt_5_4()),
        "gpt-5.4-mini" => Some(ModelProfile::gpt_5_4_mini()),
        "gpt-4.1-mini" => Some(ModelProfile::gpt_4_1_mini()),
        _ => None,
    }
}

fn accepts(profile: &ModelProfile, level: ReasoningLevel) -> bool {
    match profile.capabilities().reasoning_levels() {
        Accepted::Any => true,
        Accepted::Only(levels) => levels.contains(&level),
    }
}

fn first_accepted(profile: &ModelProfile, wanted: &[ReasoningLevel]) -> ReasoningLevel {
    wanted
        .iter()
        .copied()
        .find(|level| accepts(profile, *level))
        .unwrap_or(ReasoningLevel::Off)
}

impl Setup {
    pub fn from_environment() -> Result<Self, Failure> {
        let key = Secret::new(required(KEY_VARIABLE)?)?;
        let base = variable(BASE_VARIABLE)
            .map(|text| base_endpoint(&text))
            .transpose()?;
        let model = variable(MODEL_VARIABLE).unwrap_or_else(|| DEFAULT_MODEL.to_owned());
        let profile = preset_of(&model).ok_or_else(|| {
            format!("{MODEL_VARIABLE} names the model {model}, which has no preset in this library")
        })?;
        let http = HttpClient::new(
            HttpConfig::new(
                Duration::from_secs(10),
                Duration::from_secs(60),
                Duration::from_secs(120),
            )
            .with_total_timeout(Duration::from_secs(300)),
        )?;
        Ok(Self {
            key,
            base,
            model,
            profile,
            http,
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn default_client(&self) -> Result<OpenAiClient, Failure> {
        self.client(&self.model, &self.profile)
    }

    pub fn profile(&self) -> &ModelProfile {
        &self.profile
    }

    pub fn thinking_level(&self, profile: &ModelProfile) -> ReasoningLevel {
        first_accepted(profile, &[ReasoningLevel::Medium, ReasoningLevel::Low])
    }

    pub fn client(&self, model: &str, profile: &ModelProfile) -> Result<OpenAiClient, Failure> {
        let level = first_accepted(profile, &[ReasoningLevel::Low]);
        self.build(model, profile, level)
    }

    pub fn client_at(
        &self,
        profile: &ModelProfile,
        level: ReasoningLevel,
    ) -> Result<OpenAiClient, Failure> {
        self.build(&self.model, profile, level)
    }

    fn build(
        &self,
        model: &str,
        profile: &ModelProfile,
        level: ReasoningLevel,
    ) -> Result<OpenAiClient, Failure> {
        let settings = ClientSettings::new(
            ModelName::new(model)?,
            level,
            CacheRequirement::NotRequired,
            MaxOutputTokens::new(MAX_OUTPUT_TOKENS)?,
        );
        let config = OpenAiConfig::new(self.key.clone(), profile.clone());
        let config = match &self.base {
            Some(base) => config.with_base(base.clone()),
            None => config,
        };
        Ok(OpenAiClient::new(settings, config, self.http.clone())?)
    }
}
