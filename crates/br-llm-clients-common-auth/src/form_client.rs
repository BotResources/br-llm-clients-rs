use std::fmt;

use br_llm_clients_common_http::{AuthorizerError, Endpoint, HttpClient, NoAuthorizer, Request};

use crate::form_body::encode;
use crate::reply_parse::{error_code, parse_reply};
use crate::token_form::TokenForm;
use crate::token_reply::TokenReply;

const BODY_LIMIT: usize = 64 * 1024;
const CONTENT_TYPE: &str = "application/x-www-form-urlencoded";
const REQUEST_NOT_BUILT: &str = "the token request could not be built";
const SEND_FAILED: &str = "the token request could not be sent";
const READ_FAILED: &str = "the token answer could not be read";

#[derive(Clone)]
pub struct FormTokenClient {
    http: HttpClient,
    endpoint: Endpoint,
}

impl FormTokenClient {
    pub fn new(http: HttpClient, endpoint: Endpoint) -> Self {
        Self { http, endpoint }
    }

    pub async fn fetch(&self, form: &TokenForm) -> Result<TokenReply, AuthorizerError> {
        let request = Request::new(self.endpoint.clone(), encode(form))
            .with_header("content-type", CONTENT_TYPE)
            .map_err(|_| AuthorizerError::fetch_failed(REQUEST_NOT_BUILT))?;
        let mut response = self
            .http
            .send(request, &NoAuthorizer)
            .await
            .map_err(|_| AuthorizerError::fetch_failed(SEND_FAILED))?;
        let status = response.status();
        let body = response.body_mut().collect(BODY_LIMIT).await;
        if (200..300).contains(&status) {
            let collected = body.map_err(|_| AuthorizerError::fetch_failed(READ_FAILED))?;
            if collected.cut {
                return Err(malformed(status, "a body of more than 64 KiB"));
            }
            return parse_reply(&collected.bytes).map_err(|flaw| malformed(status, flaw.text()));
        }
        let code = body
            .ok()
            .filter(|collected| !collected.cut)
            .and_then(|collected| error_code(&collected.bytes))
            .filter(|code| !form.echoes(code));
        Err(status_error(status, code))
    }
}

fn malformed(status: u16, flaw: &str) -> AuthorizerError {
    AuthorizerError::fetch_failed(format!("the token endpoint answered {status} with {flaw}"))
}

fn status_error(status: u16, code: Option<String>) -> AuthorizerError {
    let mut text = format!("the token endpoint answered {status}");
    if let Some(code) = code {
        text.push_str(": ");
        text.push_str(&code);
    }
    if status == 429 || (500..600).contains(&status) {
        AuthorizerError::fetch_failed(text)
    } else {
        AuthorizerError::unavailable(text)
    }
}

impl fmt::Debug for FormTokenClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FormTokenClient")
            .field("endpoint", &self.endpoint)
            .finish_non_exhaustive()
    }
}
