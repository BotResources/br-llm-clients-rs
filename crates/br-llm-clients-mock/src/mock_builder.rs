use br_llm_clients_core::ClientSettings;

use crate::capabilities::Capabilities;
use crate::mock_client::MockClient;
use crate::reply_source::ReplySource;
use crate::rule::Rule;
use crate::shared::Shared;

#[derive(Debug)]
#[must_use]
pub struct MockBuilder {
    settings: ClientSettings,
    capabilities: Capabilities,
    replies: Vec<ReplySource>,
    rules: Vec<Rule>,
    fallback: Option<ReplySource>,
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

    pub fn log(mut self, enabled: bool) -> Self {
        self.log = enabled;
        self
    }

    pub fn build(self) -> MockClient {
        let shared = Shared::new(self.capabilities, self.log);
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
