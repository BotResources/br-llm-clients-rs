use std::future::Future;
use std::pin::Pin;

use crate::call_outcome::CallOutcome;
use crate::client_request::ClientRequest;
use crate::sink::StreamSink;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Client: Send + Sync {
    fn call<'a>(
        &'a self,
        request: &'a ClientRequest,
        sink: &'a mut dyn StreamSink,
    ) -> BoxFuture<'a, CallOutcome>;
}
