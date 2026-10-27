use std::fmt;

use br_llm_clients_core::{Capabilities, ClientSettings, IdSource};

use crate::mock_client::MockClient;
use crate::reply_source::ReplySource;
use crate::rule::Rule;
use crate::sequential_id_source::SequentialIdSource;
use crate::shared::Shared;

#[must_use]
pub struct MockBuilder {
    settings: ClientSettings,
    capabilities: Capabilities,
    replies: Vec<ReplySource>,
    rules: Vec<Rule>,
    fallback: Option<ReplySource>,
    id_source: Option<Box<dyn IdSource>>,
    log: bool,
}

impl MockBuilder {
    pub(crate) fn new(settings: ClientSettings, capabilities: Capabilities) -> Self {
        Self {
            settings,
            capabilities,
            replies: Vec::new(),
            rules: Vec::new(),
            fallback: None,
            id_source: None,
            log: true,
        }
    }

    pub fn reply(mut self, reply: impl Into<ReplySource>) -> Self {
        self.replies.push(reply.into());
        self
    }

    pub fn rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn fallback(mut self, fallback: impl Into<ReplySource>) -> Self {
        self.fallback = Some(fallback.into());
        self
    }

    pub fn id_source(mut self, source: impl IdSource + 'static) -> Self {
        self.id_source = Some(Box::new(source));
        self
    }

    pub fn log(mut self, enabled: bool) -> Self {
        self.log = enabled;
        self
    }

    pub fn build(self) -> MockClient {
        let id_source = self
            .id_source
            .unwrap_or_else(|| Box::new(SequentialIdSource::of_mock()));
        let shared = Shared::new(self.capabilities, id_source, self.log);
        shared.with(|state| {
            for reply in self.replies {
                state.script.push_entry(reply);
            }
            for rule in self.rules {
                state.script.push_rule(rule);
            }
            state.script.replace_fallback(self.fallback);
        });
        MockClient::from_parts(shared, self.settings)
    }
}

impl fmt::Debug for MockBuilder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MockBuilder")
            .field("settings", &self.settings)
            .field("replies", &self.replies)
            .field("rules", &self.rules)
            .field("fallback", &self.fallback)
            .field("log", &self.log)
            .finish_non_exhaustive()
    }
}
