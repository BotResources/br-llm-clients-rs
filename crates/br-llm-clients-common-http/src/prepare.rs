use bytes::Bytes;
use http::header::{ACCEPT_ENCODING, CONTENT_LENGTH, HOST, USER_AGENT};
use http::{HeaderName, HeaderValue};

use crate::endpoint::Endpoint;
use crate::request::Request;
use crate::send_error::SendError;

pub(crate) struct Prepared {
    pub(crate) endpoint: Endpoint,
    pub(crate) host: String,
    pub(crate) path_and_query: String,
    pub(crate) headers: Vec<(HeaderName, HeaderValue)>,
    pub(crate) body: Bytes,
}

pub(crate) fn prepare(request: Request, user_agent: &HeaderValue) -> Result<Prepared, SendError> {
    let (endpoint, own_headers, body) = request.into_parts();
    let host = endpoint.host_header();
    let host_value = HeaderValue::from_str(&host).map_err(|_| SendError::Protocol {
        text: "the endpoint host cannot be sent as a header value".to_owned(),
    })?;
    let mut headers = vec![(HOST, host_value)];
    headers.push((USER_AGENT, user_agent.clone()));
    headers.push((ACCEPT_ENCODING, HeaderValue::from_static("identity")));
    headers.push((CONTENT_LENGTH, HeaderValue::from(body.len())));
    headers.extend(own_headers);
    Ok(Prepared {
        path_and_query: endpoint.path_and_query(),
        host,
        endpoint,
        headers,
        body,
    })
}
