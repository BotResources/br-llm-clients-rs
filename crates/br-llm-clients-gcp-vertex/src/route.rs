use br_llm_clients_common_call::Endpoint;

use crate::build_error::VertexBuildError;

const GLOBAL: &str = "global";
const MULTI_REGIONS: [&str; 2] = ["us", "eu"];
const VERB: &str = "streamRawPredict";

pub(crate) fn predict_endpoint(
    project: &str,
    location: &str,
    model: &str,
    base: Option<Endpoint>,
) -> Result<Endpoint, VertexBuildError> {
    let base = match base {
        Some(base) if base.is_https() => base,
        Some(_) => {
            return Err(VertexBuildError::endpoint(
                "the base must be https because the bearer token has the cloud-platform scope",
            ));
        }
        None => default_base(location)?,
    };
    let model_and_verb = format!("{model}:{VERB}");
    let route = [
        "v1",
        "projects",
        project,
        "locations",
        location,
        "publishers",
        "anthropic",
        "models",
        model_and_verb.as_str(),
    ];
    route.iter().try_fold(base, |endpoint, segment| {
        endpoint.join_segment(segment).map_err(|_| {
            VertexBuildError::endpoint(
                "the route of the model cannot be joined to the base: the endpoint would pass 8192 bytes",
            )
        })
    })
}

fn default_base(location: &str) -> Result<Endpoint, VertexBuildError> {
    let base = if location == GLOBAL {
        "https://aiplatform.googleapis.com/".to_owned()
    } else if MULTI_REGIONS.contains(&location) {
        format!("https://aiplatform.{location}.rep.googleapis.com/")
    } else {
        format!("https://{location}-aiplatform.googleapis.com/")
    };
    Endpoint::https(&base).map_err(|_| {
        VertexBuildError::endpoint("the default base of the location is not a valid endpoint")
    })
}
