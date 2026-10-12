use std::fmt;
use std::sync::Arc;

use crate::build_error::MockBuildError;
use crate::call_view::MatchContext;
use crate::reply_source::ReplySource;

pub(crate) type Matcher = dyn Fn(&MatchContext<'_>) -> bool + Send + Sync + 'static;

#[derive(Clone)]
#[must_use = "a rule does nothing until it is pushed to a mock"]
pub struct Rule {
    pub(crate) matcher: Arc<Matcher>,
    pub(crate) reply: ReplySource,
    pub(crate) limit: Option<usize>,
}

impl Rule {
    pub fn new<M>(matcher: M, reply: impl Into<ReplySource>) -> Self
    where
        M: Fn(&MatchContext<'_>) -> bool + Send + Sync + 'static,
    {
        Self {
            matcher: Arc::new(matcher),
            reply: reply.into(),
            limit: None,
        }
    }

    pub fn always(reply: impl Into<ReplySource>) -> Self {
        Self::new(|_| true, reply)
    }

    pub fn with_limit(mut self, uses: usize) -> Result<Self, MockBuildError> {
        if uses == 0 {
            return Err(MockBuildError::RuleLimitZero);
        }
        self.limit = Some(uses);
        Ok(self)
    }
}

impl fmt::Debug for Rule {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Rule")
            .field("reply", &self.reply)
            .field("limit", &self.limit)
            .finish_non_exhaustive()
    }
}
