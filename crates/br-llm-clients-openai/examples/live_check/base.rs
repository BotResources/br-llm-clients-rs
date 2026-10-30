use br_llm_clients_common_http::Endpoint;

use crate::Failure;

pub const BASE_VARIABLE: &str = "OPENAI_BASE_URL";

const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

fn is_loopback(host: &str) -> bool {
    LOOPBACK_HOSTS
        .iter()
        .any(|loopback| host.eq_ignore_ascii_case(loopback))
}

fn ends_with_version(path: &str) -> bool {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .is_some_and(|segment| segment.eq_ignore_ascii_case("v1"))
}

pub fn base_endpoint(text: &str) -> Result<Endpoint, Failure> {
    let scheme = text.split_once("://").map(|(scheme, _)| scheme);
    let endpoint = match scheme {
        Some(scheme) if scheme.eq_ignore_ascii_case("https") => Endpoint::https(text)?,
        Some(scheme) if scheme.eq_ignore_ascii_case("http") => {
            let endpoint = Endpoint::plain_http(text)?;
            if !is_loopback(endpoint.host()) {
                return Err(format!(
                    "{BASE_VARIABLE} may use http:// only for localhost, 127.0.0.1 or [::1]; use https:// for any other host"
                )
                .into());
            }
            endpoint
        }
        _ => return Err(format!("{BASE_VARIABLE} must start with https:// or http://").into()),
    };
    if endpoint.query().is_some() {
        return Err(format!("{BASE_VARIABLE} must not hold a query").into());
    }
    if ends_with_version(endpoint.path()) {
        return Err(format!(
            "{BASE_VARIABLE} must not end with /v1: the library adds /v1/responses itself, so give the origin, with an optional path prefix, without /v1"
        )
        .into());
    }
    Ok(endpoint)
}
