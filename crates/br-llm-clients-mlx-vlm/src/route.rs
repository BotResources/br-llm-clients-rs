use br_llm_clients_common_http::Endpoint;

use crate::build_error::MlxVlmBuildError;

const ROUTE: [&str; 3] = ["v1", "chat", "completions"];

pub(crate) fn route_endpoint(base: &Endpoint) -> Result<Endpoint, MlxVlmBuildError> {
    ROUTE
        .iter()
        .try_fold(base.clone(), |endpoint, segment| {
            endpoint.join_segment(segment)
        })
        .map_err(|_| MlxVlmBuildError::InvalidEndpoint {
            reason: "the endpoint of the route is longer than the limit of an endpoint".to_owned(),
        })
}
