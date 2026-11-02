use br_llm_clients_common_call::Endpoint;
use br_llm_clients_common_http::ConfigError;

use crate::api::Route;
use crate::base::check_base;
use crate::build_error::AzureOpenAiBuildError;

pub(crate) const API_VERSION_QUERY: &str = "api-version";
const V1_CHAT_COMPLETIONS: [&str; 4] = ["openai", "v1", "chat", "completions"];
const V1_RESPONSES: [&str; 3] = ["openai", "v1", "responses"];
const DATED_DEPLOYMENTS: [&str; 2] = ["openai", "deployments"];
const DATED_CHAT_COMPLETIONS: [&str; 2] = ["chat", "completions"];
const DATED_RESPONSES: [&str; 2] = ["openai", "responses"];

fn invalid(error: ConfigError) -> AzureOpenAiBuildError {
    let reason = match error {
        ConfigError::InvalidEndpoint { text } => text,
        other => other.to_string(),
    };
    AzureOpenAiBuildError::InvalidEndpoint { reason }
}

fn join_all(endpoint: Endpoint, segments: &[&str]) -> Result<Endpoint, ConfigError> {
    segments
        .iter()
        .try_fold(endpoint, |joined, segment| joined.join_segment(segment))
}

fn dated_path(base: Endpoint, route: Route, deployment: &str) -> Result<Endpoint, ConfigError> {
    match route {
        Route::ChatCompletions => {
            let deployments = join_all(base, &DATED_DEPLOYMENTS)?;
            join_all(
                deployments.join_segment(deployment)?,
                &DATED_CHAT_COMPLETIONS,
            )
        }
        Route::Responses => join_all(base, &DATED_RESPONSES),
    }
}

pub(crate) fn route_endpoint(
    base: Endpoint,
    route: Route,
    api_version: Option<&str>,
    deployment: &str,
) -> Result<Endpoint, AzureOpenAiBuildError> {
    check_base(&base)?;
    let endpoint = match (api_version, route) {
        (None, Route::ChatCompletions) => join_all(base, &V1_CHAT_COMPLETIONS),
        (None, Route::Responses) => join_all(base, &V1_RESPONSES),
        (Some(version), route) => dated_path(base, route, deployment)
            .and_then(|path| path.with_query_pair(API_VERSION_QUERY, version)),
    };
    endpoint.map_err(invalid)
}
