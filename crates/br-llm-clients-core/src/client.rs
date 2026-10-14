use std::future::Future;
use std::pin::Pin;

use crate::call_outcome::CallOutcome;
use crate::client_request::ClientRequest;
use crate::client_settings::ClientSettings;
use crate::sink::StreamSink;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Client: Send + Sync {
    fn call<'a>(
        &'a self,
        request: &'a ClientRequest,
        sink: &'a mut dyn StreamSink,
    ) -> BoxFuture<'a, CallOutcome>;

    /// The `ClientSettings` this client is bound to. Pure: not async, no I/O.
    /// It gives the common settings only; the configuration of a provider
    /// library stays private to that library.
    fn settings(&self) -> &ClientSettings;
}
