use std::sync::Arc;

use br_llm_clients_aws_bedrock::{
    AwsCredentials, BedrockClient, BedrockConfig, BedrockCredential, Endpoint, HttpClient, Secret,
    StaticCredentials,
};
use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens, ReasoningLevel};
use br_llm_messages::ModelName;

use crate::fail;

const DEFAULT_MODEL: &str = "eu.anthropic.claude-haiku-4-5-20251001-v1:0";
const MAX_OUTPUT_TOKENS: u32 = 4096;
const PROFILE_PREFIXES: [&str; 7] = ["global.", "us.", "eu.", "au.", "jp.", "apac.", "us-gov."];
const PROVIDER_PREFIX: &str = "anthropic.";
const ALWAYS_THINKING: [&str; 3] = ["claude-fable-5-1", "claude-opus-5-5", "claude-sonnet-5-5"];
const API_KEY: &str = "AWS_BEARER_TOKEN_BEDROCK";
const ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const SESSION_TOKEN: &str = "AWS_SESSION_TOKEN";

pub struct Path {
    pub label: &'static str,
    credential: BedrockCredential,
}

pub struct Setup {
    pub model: String,
    pub paths: Vec<Path>,
    region: String,
    base: Option<Endpoint>,
}

impl Setup {
    pub fn client(
        &self,
        path: &Path,
        model: &str,
        http: &HttpClient,
    ) -> Result<BedrockClient, String> {
        let settings = ClientSettings::new(
            ModelName::new(model.to_owned()).map_err(fail)?,
            reasoning(model),
            CacheRequirement::NotRequired,
            MaxOutputTokens::new(MAX_OUTPUT_TOKENS).map_err(fail)?,
        );
        let config = BedrockConfig::new(self.region.clone(), path.credential.clone());
        let config = match &self.base {
            Some(base) => config.with_base(base.clone()),
            None => config,
        };
        BedrockClient::new(settings, config, http.clone()).map_err(fail)
    }
}

fn base_name(model: &str) -> &str {
    let unprefixed = PROFILE_PREFIXES
        .iter()
        .find_map(|prefix| model.strip_prefix(prefix))
        .unwrap_or(model);
    unprefixed
        .strip_prefix(PROVIDER_PREFIX)
        .unwrap_or(unprefixed)
}

fn reasoning(model: &str) -> ReasoningLevel {
    if ALWAYS_THINKING.contains(&base_name(model)) {
        ReasoningLevel::Low
    } else {
        ReasoningLevel::Off
    }
}

pub fn setup() -> Result<Setup, String> {
    let region = required("AWS_REGION")?;
    let model = optional("BEDROCK_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_owned());
    let paths = paths()?;
    let base = optional("BEDROCK_ENDPOINT").map(base).transpose()?;
    Ok(Setup {
        model,
        paths,
        region,
        base,
    })
}

fn paths() -> Result<Vec<Path>, String> {
    let mut paths = Vec::new();
    if optional(ACCESS_KEY_ID).is_some() || optional(SECRET_ACCESS_KEY).is_some() {
        paths.push(Path {
            label: "sigv4",
            credential: sigv4()?,
        });
    }
    if let Some(token) = optional(API_KEY) {
        let key = Secret::new(token).map_err(|_| format!("{API_KEY} is empty"))?;
        paths.push(Path {
            label: "api-key",
            credential: BedrockCredential::ApiKey(key),
        });
    }
    if paths.is_empty() {
        return Err(format!(
            "no credential: set {API_KEY}, or {ACCESS_KEY_ID} and {SECRET_ACCESS_KEY} (and {SESSION_TOKEN} for temporary credentials), or both"
        ));
    }
    Ok(paths)
}

fn sigv4() -> Result<BedrockCredential, String> {
    let secret_access_key = Secret::new(required(SECRET_ACCESS_KEY)?)
        .map_err(|_| format!("{SECRET_ACCESS_KEY} is empty"))?;
    let credentials = AwsCredentials::new(required(ACCESS_KEY_ID)?, secret_access_key)
        .map_err(|_| format!("{ACCESS_KEY_ID} is not a valid access key id"))?;
    let credentials = match optional(SESSION_TOKEN) {
        Some(token) => credentials.with_session_token(
            Secret::new(token).map_err(|_| format!("{SESSION_TOKEN} is empty"))?,
        ),
        None => credentials,
    };
    Ok(BedrockCredential::Sigv4(Arc::new(StaticCredentials::new(
        credentials,
    ))))
}

fn base(text: String) -> Result<Endpoint, String> {
    let parsed = if text.starts_with("http://") {
        Endpoint::plain_http(&text)
    } else {
        Endpoint::https(&text)
    };
    parsed.map_err(|error| format!("BEDROCK_ENDPOINT is not a valid base: {error}"))
}

fn required(name: &str) -> Result<String, String> {
    optional(name).ok_or_else(|| format!("the environment variable {name} is not set or is empty"))
}

fn optional(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}
