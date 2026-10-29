use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens, ReasoningLevel};
use br_llm_clients_gcp_vertex::{
    HttpClient, ServiceAccountKey, VertexClient, VertexConfig, VertexCredential,
};
use br_llm_messages::ModelName;

use crate::fail;

const MAX_OUTPUT_TOKENS: u32 = 4096;
const ALWAYS_THINKING: [&str; 3] = ["claude-fable-5-1", "claude-opus-5-5", "claude-sonnet-5-5"];
const PROJECT: &str = "VERTEX_AI_PROJECT_ID";
const LOCATION: &str = "VERTEX_AI_LOCATION";
const MODEL: &str = "VERTEX_AI_MODEL";
const CREDENTIALS_FILE: &str = "VERTEX_AI_CREDENTIALS_FILE";

pub struct Setup {
    pub model: String,
    project: String,
    location: String,
    credential: VertexCredential,
}

impl Setup {
    pub fn client(&self, model: &str, http: &HttpClient) -> Result<VertexClient, String> {
        let settings = ClientSettings::new(
            ModelName::new(model.to_owned()).map_err(fail)?,
            reasoning(model),
            CacheRequirement::NotRequired,
            MaxOutputTokens::new(MAX_OUTPUT_TOKENS).map_err(fail)?,
        );
        let config = VertexConfig::new(
            self.project.clone(),
            self.location.clone(),
            self.credential.clone(),
        );
        VertexClient::new(settings, config, http.clone()).map_err(fail)
    }
}

fn reasoning(model: &str) -> ReasoningLevel {
    if ALWAYS_THINKING.contains(&model) {
        ReasoningLevel::Low
    } else {
        ReasoningLevel::Off
    }
}

pub fn setup() -> Result<Setup, String> {
    let project = required(PROJECT)?;
    let location = required(LOCATION)?;
    let model = required(MODEL)?;
    let path = required(CREDENTIALS_FILE)?;
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("the file named by {CREDENTIALS_FILE} cannot be read: {error}"))?;
    let key = ServiceAccountKey::from_json(&text).map_err(|error| {
        format!("the file named by {CREDENTIALS_FILE} is not a usable service account key: {error}")
    })?;
    Ok(Setup {
        model,
        project,
        location,
        credential: VertexCredential::ServiceAccount(key),
    })
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("the environment variable {name} is not set or is empty"))
}
