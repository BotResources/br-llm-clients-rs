use std::fmt;
use std::sync::Arc;

use crate::build_error::MockBuildError;
use crate::call_view::ReplyContext;
use crate::reply::Reply;

pub(crate) type ReplyFn =
    dyn Fn(&ReplyContext<'_>) -> Result<Reply, MockBuildError> + Send + Sync + 'static;

#[derive(Clone)]
pub(crate) enum Repr {
    Value(Arc<Reply>),
    Function(Arc<ReplyFn>),
}

#[derive(Clone)]
#[must_use = "a reply source does nothing until it is given to a mock"]
pub struct ReplySource {
    pub(crate) repr: Repr,
}

impl ReplySource {
    pub fn value(reply: Reply) -> Self {
        Self {
            repr: Repr::Value(Arc::new(reply)),
        }
    }

    pub fn function<F>(function: F) -> Self
    where
        F: Fn(&ReplyContext<'_>) -> Result<Reply, MockBuildError> + Send + Sync + 'static,
    {
        Self {
            repr: Repr::Function(Arc::new(function)),
        }
    }
}

impl From<Reply> for ReplySource {
    fn from(reply: Reply) -> Self {
        Self::value(reply)
    }
}

impl fmt::Debug for ReplySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Value(reply) => formatter
                .debug_tuple("ReplySource::Value")
                .field(reply)
                .finish(),
            Repr::Function(_) => formatter.write_str("ReplySource::Function"),
        }
    }
}
