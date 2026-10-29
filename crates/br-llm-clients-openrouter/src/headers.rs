use br_llm_clients_common_http::{ConfigError, Endpoint, Request};

use crate::build_error::OpenRouterBuildError;

const REFERER_HEADER: &str = "HTTP-Referer";
const TITLE_HEADER: &str = "X-OpenRouter-Title";
const LEGACY_TITLE_HEADER: &str = "X-Title";
const REFUSED_NAMES: [&str; 7] = [
    "accept",
    "authorization",
    "content-type",
    "anthropic-version",
    REFERER_HEADER,
    TITLE_HEADER,
    LEGACY_TITLE_HEADER,
];
const PROBE_NAME: &str = "x-openrouter-probe";
const PROBE_VALUE: &str = "probe";
const REFUSED_NAME: &str = "has a name that the library refuses: accept, authorization, content-type, anthropic-version, http-referer, x-openrouter-title or x-title";
const INVALID_VALUE: &str = "has a value that is not a valid header value";

pub(crate) struct Attribution<'a> {
    pub(crate) referer: Option<&'a str>,
    pub(crate) title: Option<&'a str>,
}

fn invalid_extra(index: usize, fault: &str) -> OpenRouterBuildError {
    OpenRouterBuildError::InvalidExtraHeader {
        reason: format!("the extra header at index {index} (counted from 0) {fault}"),
    }
}

fn invalid_attribution(what: &str, fault: &str) -> OpenRouterBuildError {
    OpenRouterBuildError::InvalidAttribution {
        reason: format!("the {what} {fault}"),
    }
}

fn name_fault(error: &ConfigError) -> &'static str {
    match error {
        ConfigError::DuplicateHeader { .. } => "has a name that is given twice",
        ConfigError::ReservedHeader { .. } => "has a name that the HTTP library manages",
        _ => "has a name that is not a valid header name",
    }
}

fn is_valid_value(base: &Endpoint, value: &str) -> bool {
    Request::new(base.clone(), Vec::new())
        .with_header(PROBE_NAME, value)
        .is_ok()
}

fn attribution_header(
    base: &Endpoint,
    name: &str,
    what: &str,
    value: Option<&str>,
) -> Result<Option<(String, String)>, OpenRouterBuildError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Err(invalid_attribution(what, "is blank"));
    }
    if !is_valid_value(base, value) {
        return Err(invalid_attribution(what, "is not a valid header value"));
    }
    Ok(Some((name.to_owned(), value.to_owned())))
}

pub(crate) fn request_headers(
    base: &Endpoint,
    attribution: &Attribution<'_>,
    extra: Vec<(String, String)>,
) -> Result<Vec<(String, String)>, OpenRouterBuildError> {
    let mut headers = Vec::new();
    headers.extend(attribution_header(
        base,
        REFERER_HEADER,
        "app referer",
        attribution.referer,
    )?);
    headers.extend(attribution_header(
        base,
        TITLE_HEADER,
        "app title",
        attribution.title,
    )?);
    let mut names = Request::new(base.clone(), Vec::new());
    for (index, (name, value)) in extra.iter().enumerate() {
        if REFUSED_NAMES
            .iter()
            .any(|refused| refused.eq_ignore_ascii_case(name))
        {
            return Err(invalid_extra(index, REFUSED_NAME));
        }
        names = names
            .with_header(name, PROBE_VALUE)
            .map_err(|error| invalid_extra(index, name_fault(&error)))?;
        if !is_valid_value(base, value) {
            return Err(invalid_extra(index, INVALID_VALUE));
        }
    }
    headers.extend(extra);
    Ok(headers)
}
