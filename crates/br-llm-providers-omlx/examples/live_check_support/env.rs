use std::env;

use br_llm_clients_common_http::{Endpoint, Secret};

use crate::Failure;

pub fn required<const N: usize>(names: [&str; N]) -> Result<[String; N], Failure> {
    let values = names.map(optional);
    let missing: Vec<&str> = names
        .iter()
        .zip(&values)
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| *name)
        .collect();
    if !missing.is_empty() {
        return Err(format!("missing environment variable(s): {}", missing.join(", ")).into());
    }
    Ok(values.map(Option::unwrap_or_default))
}

pub fn optional(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

pub fn key(name: &str, text: String) -> Result<Secret, Failure> {
    Secret::new(text).map_err(|error| format!("{name} is refused: {error}").into())
}

pub fn base_endpoint(name: &str, text: &str) -> Result<Endpoint, Failure> {
    let scheme = text
        .split_once("://")
        .map(|(scheme, _)| scheme.to_ascii_lowercase());
    let endpoint = match scheme.as_deref() {
        Some("https") => Endpoint::https(text),
        Some("http") => Endpoint::plain_http(text),
        _ => return Err(format!("{name} must start with https:// or http://").into()),
    }
    .map_err(|error| format!("{name} is not a valid base URL: {error}"))?;
    if endpoint.path().trim_end_matches('/').ends_with("/v1") {
        return Err(format!(
            "{name} must be the root of the server, without `/v1`: the library adds `v1` to the route"
        )
        .into());
    }
    Ok(endpoint)
}
