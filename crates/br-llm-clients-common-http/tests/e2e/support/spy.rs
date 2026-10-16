use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_common_http::{
    AuthorizationRequest, AuthorizedHeader, Authorizer, AuthorizerError, BoxFuture, Secret,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub method: String,
    pub path_and_query: String,
    pub host: String,
    pub headers: Vec<(String, Vec<u8>)>,
    pub body: Vec<u8>,
}

pub struct Spy {
    result: Result<Vec<(String, String)>, AuthorizerError>,
    calls: AtomicUsize,
    seen: Mutex<Vec<Seen>>,
}

impl Spy {
    pub fn returning(headers: &[(&str, &str)]) -> Self {
        Self {
            result: Ok(headers
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                .collect()),
            calls: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
        }
    }

    pub fn failing(error: AuthorizerError) -> Self {
        Self {
            result: Err(error),
            calls: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub fn seen(&self) -> Vec<Seen> {
        self.seen.lock().expect("lock").clone()
    }
}

impl Authorizer for Spy {
    fn authorize<'a>(
        &'a self,
        request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().expect("lock").push(Seen {
            method: request.method().to_owned(),
            path_and_query: request.path_and_query().to_owned(),
            host: request.host().to_owned(),
            headers: request
                .headers()
                .map(|(n, v)| (n.to_owned(), v.to_vec()))
                .collect(),
            body: request.body().to_vec(),
        });
        let result = match &self.result {
            Ok(headers) => Ok(headers
                .iter()
                .map(|(n, v)| {
                    AuthorizedHeader::new(n.clone(), Secret::new(v.clone()).expect("secret"))
                })
                .collect()),
            Err(error) => Err(error.clone()),
        };
        Box::pin(async move { result })
    }
}
