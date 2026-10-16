use std::fmt;
use std::sync::Arc;

use br_llm_clients_core::{
    BoxFuture, CallOutcome, Client, ClientRequest, ClientSettings, StreamSink,
};

use crate::call::drive;
use crate::call_log::{LoggedCall, StateKind};
use crate::capabilities::Capabilities;
use crate::log_disabled::LogDisabled;
use crate::mock_builder::MockBuilder;
use crate::mock_waits::{WaitForCalls, WaitForState};
use crate::reply_source::ReplySource;
use crate::rule::Rule;
use crate::shared::Shared;

#[derive(Clone)]
pub struct MockClient {
    shared: Arc<Shared>,
    settings: ClientSettings,
}

impl MockClient {
    pub fn new(settings: ClientSettings, capabilities: Capabilities) -> Self {
        Self::builder(settings, capabilities).build()
    }

    pub fn builder(settings: ClientSettings, capabilities: Capabilities) -> MockBuilder {
        MockBuilder::new(settings, capabilities)
    }

    pub(crate) fn from_parts(shared: Arc<Shared>, settings: ClientSettings) -> Self {
        Self { shared, settings }
    }

    #[must_use = "the new handle is the only result"]
    pub fn with_settings(&self, settings: ClientSettings) -> MockClient {
        MockClient {
            shared: Arc::clone(&self.shared),
            settings,
        }
    }

    pub fn push_reply(&self, reply: impl Into<ReplySource>) {
        let reply = reply.into();
        self.shared.with(|state| state.script.push_entry(reply));
    }

    pub fn push_rule(&self, rule: Rule) {
        self.shared.with(|state| state.script.push_rule(rule));
    }

    pub fn set_fallback(&self, fallback: impl Into<ReplySource>) {
        let fallback = fallback.into();
        let replaced = self
            .shared
            .with(|state| state.script.replace_fallback(Some(fallback)));
        drop(replaced);
    }

    pub fn remove_fallback(&self) {
        let replaced = self
            .shared
            .with(|state| state.script.replace_fallback(None));
        drop(replaced);
    }

    pub fn log(&self) -> Vec<LoggedCall> {
        self.shared.with(|state| state.log.snapshot())
    }

    pub fn calls(&self) -> usize {
        self.shared.with(|state| state.log.began())
    }

    pub fn unused_entries(&self) -> usize {
        self.shared.with(|state| state.script.unused_entries())
    }

    pub fn unused_uses(&self) -> usize {
        self.shared.with(|state| state.script.unused_uses())
    }

    pub fn wait_for_calls(&self, count: usize) -> WaitForCalls {
        WaitForCalls::new(Arc::clone(&self.shared), count)
    }

    pub fn wait_for_state(
        &self,
        kind: StateKind,
        count: usize,
    ) -> Result<WaitForState, LogDisabled> {
        if !self.shared.log_enabled() {
            return Err(LogDisabled);
        }
        Ok(WaitForState::new(Arc::clone(&self.shared), kind, count))
    }
}

impl fmt::Debug for MockClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MockClient")
            .field("settings", &self.settings)
            .field("calls", &self.calls())
            .finish_non_exhaustive()
    }
}

impl Client for MockClient {
    fn call<'a>(
        &'a self,
        request: &'a ClientRequest,
        sink: &'a mut dyn StreamSink,
    ) -> BoxFuture<'a, CallOutcome> {
        Box::pin(drive(&self.shared, &self.settings, request, sink))
    }

    fn settings(&self) -> &ClientSettings {
        &self.settings
    }
}
