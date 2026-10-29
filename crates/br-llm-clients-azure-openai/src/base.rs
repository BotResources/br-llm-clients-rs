use br_llm_clients_common_call::Endpoint;

use crate::build_error::AzureOpenAiBuildError;
use crate::route::API_VERSION_QUERY;

const OPENAI_SEGMENT: &str = "openai";
const V1_SEGMENT: &str = "v1";
const ROUTE_PREFIX_REASON: &str = "the endpoint path ends with `openai` or `openai/v1`: give the address of the resource, the client adds the path of the route itself";
const API_VERSION_REASON: &str = "the endpoint query holds `api-version`: give the api version in the config, the client adds the query itself";

fn ends_with_route_prefix(path: &str) -> bool {
    let mut segments = path.trim_end_matches('/').rsplit('/');
    matches!(
        (segments.next(), segments.next()),
        (Some(OPENAI_SEGMENT), _) | (Some(V1_SEGMENT), Some(OPENAI_SEGMENT))
    )
}

fn holds_api_version(query: Option<&str>) -> bool {
    query.is_some_and(|query| {
        query.split('&').any(|pair| {
            pair.split('=')
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case(API_VERSION_QUERY))
        })
    })
}

pub(crate) fn check_base(base: &Endpoint) -> Result<(), AzureOpenAiBuildError> {
    if ends_with_route_prefix(base.path()) {
        return Err(AzureOpenAiBuildError::InvalidEndpoint {
            reason: ROUTE_PREFIX_REASON.to_owned(),
        });
    }
    if holds_api_version(base.query()) {
        return Err(AzureOpenAiBuildError::InvalidEndpoint {
            reason: API_VERSION_REASON.to_owned(),
        });
    }
    Ok(())
}
