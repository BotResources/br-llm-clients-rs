use std::fmt;
use std::future::Future;
use std::pin::Pin;

use http::{HeaderName, HeaderValue};

use crate::authorizer_error::AuthorizerError;
use crate::secret::Secret;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Authorizer: Send + Sync {
    fn authorize<'a>(
        &'a self,
        request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>>;
}

#[derive(Clone)]
#[non_exhaustive]
pub struct AuthorizedHeader {
    pub name: String,
    pub value: Secret,
}

impl AuthorizedHeader {
    pub fn new(name: impl Into<String>, value: Secret) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

pub struct AuthorizationRequest<'a> {
    path_and_query: &'a str,
    host: &'a str,
    headers: &'a [(HeaderName, HeaderValue)],
    body: &'a [u8],
}

impl<'a> AuthorizationRequest<'a> {
    pub(crate) fn new(
        path_and_query: &'a str,
        host: &'a str,
        headers: &'a [(HeaderName, HeaderValue)],
        body: &'a [u8],
    ) -> Self {
        Self {
            path_and_query,
            host,
            headers,
            body,
        }
    }

    pub fn method(&self) -> &str {
        "POST"
    }

    pub fn path_and_query(&self) -> &str {
        self.path_and_query
    }

    pub fn host(&self) -> &str {
        self.host
    }

    pub fn headers(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_bytes()))
    }

    pub fn header(&self, name: &str) -> Option<&[u8]> {
        self.headers
            .iter()
            .find(|(candidate, _)| candidate.as_str().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_bytes())
    }

    pub fn body(&self) -> &[u8] {
        self.body
    }
}

impl fmt::Debug for AuthorizedHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthorizedHeader")
            .field("name", &self.name)
            .field("value", &self.value)
            .finish()
    }
}

impl fmt::Debug for AuthorizationRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.headers.iter().map(|(name, _)| name.as_str()).collect();
        f.debug_struct("AuthorizationRequest")
            .field("host", &self.host)
            .field("headers", &names)
            .field("body_len", &self.body.len())
            .finish()
    }
}
