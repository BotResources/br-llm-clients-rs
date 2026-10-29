use std::env;

use br_llm_clients_openrouter::{Endpoint, Secret};

use crate::Failure;

pub fn required(name: &str) -> Result<String, Failure> {
    optional(name).ok_or_else(|| format!("missing environment variable: {name}").into())
}

pub fn optional(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

pub fn key(name: &str, text: String) -> Result<Secret, Failure> {
    Secret::new(text).map_err(|error| format!("{name} is refused: {error}").into())
}

pub fn base_endpoint(name: &str, text: &str) -> Result<Endpoint, Failure> {
    if !text.to_ascii_lowercase().starts_with("https://") {
        return Err(format!("{name} must start with https://").into());
    }
    let endpoint = Endpoint::https(text)
        .map_err(|error| format!("{name} is not a valid base URL: {error}"))?;
    if !endpoint.path().trim_end_matches('/').ends_with("/api/v1") {
        return Err(format!(
            "{name} must end with `/api/v1`, as https://openrouter.ai/api/v1 does: the library adds only the route"
        )
        .into());
    }
    Ok(endpoint)
}
