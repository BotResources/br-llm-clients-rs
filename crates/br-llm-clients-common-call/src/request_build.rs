use br_llm_clients_common_http::Request;
use br_llm_clients_core::ClientError;

use crate::call_spec::CallSpec;
use crate::encoded_request::EncodedRequest;

const ACCEPT: &str = "accept";
const EVENT_STREAM: &str = "text/event-stream";

fn rejected(origin: &str) -> ClientError {
    ClientError::ProviderRejected {
        status: None,
        code: None,
        body: format!("a header of {origin} is refused by the HTTP library or given twice"),
    }
}

fn check_list(
    spec: &CallSpec,
    headers: &[(String, String)],
    origin: &str,
) -> Result<(), ClientError> {
    headers
        .iter()
        .try_fold(
            Request::new(spec.endpoint.clone(), Vec::new()),
            |scratch, (name, value)| scratch.with_header(name, value),
        )
        .map(drop)
        .map_err(|_| rejected(origin))
}

fn same_name(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

pub(crate) fn build_request(
    spec: &CallSpec,
    encoded: &EncodedRequest,
) -> Result<Request, ClientError> {
    check_list(spec, encoded.headers(), "the dialect")?;
    check_list(spec, &spec.extra_headers, "the provider")?;
    let dialect_kept = encoded.headers().iter().filter(|(name, _)| {
        !same_name(name, ACCEPT)
            && !spec
                .extra_headers
                .iter()
                .any(|(provider, _)| same_name(provider, name))
    });
    let provider_kept = spec
        .extra_headers
        .iter()
        .filter(|(name, _)| !same_name(name, ACCEPT));
    let request = Request::new(spec.endpoint.clone(), encoded.body())
        .with_header(ACCEPT, EVENT_STREAM)
        .map_err(|_| rejected("the library"))?;
    dialect_kept
        .chain(provider_kept)
        .try_fold(request, |request, (name, value)| {
            request.with_header(name, value)
        })
        .map_err(|_| rejected("the provider"))
}
