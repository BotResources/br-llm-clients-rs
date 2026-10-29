use br_llm_clients_common_call::Endpoint;

use crate::build_error::BedrockBuildError;

const ROUTE: [&str; 3] = ["anthropic", "v1", "messages"];

pub(crate) fn messages_endpoint(
    region: &str,
    base: Option<Endpoint>,
) -> Result<Endpoint, BedrockBuildError> {
    let base = match base {
        Some(base) => base,
        None => default_base(region)?,
    };
    ROUTE.iter().try_fold(base, |endpoint, segment| {
        endpoint.join_segment(segment).map_err(|_| {
            BedrockBuildError::endpoint(
                "the route anthropic/v1/messages cannot be joined to the base: the endpoint would pass 8192 bytes",
            )
        })
    })
}

fn default_base(region: &str) -> Result<Endpoint, BedrockBuildError> {
    Endpoint::https(&format!("https://bedrock-runtime.{region}.amazonaws.com/")).map_err(|_| {
        BedrockBuildError::endpoint("the default base of the region is not a valid endpoint")
    })
}
