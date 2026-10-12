use std::sync::{Arc, Mutex, MutexGuard};

use br_llm_clients_core::{ClientRequest, ClientSettings};

use crate::capabilities::Capabilities;
use crate::log::LogState;
use crate::script::ScriptState;
use crate::sync::{lock, wake_all};
use crate::tracker::CallTracker;

pub(crate) struct State {
    pub(crate) script: ScriptState,
    pub(crate) log: LogState,
}

pub(crate) struct Shared {
    capabilities: Capabilities,
    log_enabled: bool,
    state: Mutex<State>,
}

pub(crate) struct Begun {
    pub(crate) tracker: CallTracker,
    pub(crate) rules_at_start: usize,
}

impl Shared {
    pub(crate) fn new(capabilities: Capabilities, log_enabled: bool) -> Arc<Self> {
        Arc::new(Self {
            capabilities,
            log_enabled,
            state: Mutex::new(State {
                script: ScriptState::default(),
                log: LogState::new(),
            }),
        })
    }

    pub(crate) fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub(crate) fn log_enabled(&self) -> bool {
        self.log_enabled
    }

    pub(crate) fn guard(&self) -> MutexGuard<'_, State> {
        lock(&self.state)
    }

    pub(crate) fn with<R>(&self, action: impl FnOnce(&mut State) -> R) -> R {
        action(&mut self.guard())
    }

    pub(crate) fn begin(
        self: &Arc<Self>,
        request: &ClientRequest,
        settings: &ClientSettings,
    ) -> Begun {
        let copies = self
            .log_enabled
            .then(|| (request.clone(), settings.clone()));
        let (sequence, rules_at_start, wakers) = self.with(|state| {
            let rules_at_start = state.script.rule_count();
            let (sequence, wakers) = state.log.begin(copies);
            (sequence, rules_at_start, wakers)
        });
        wake_all(wakers);
        Begun {
            tracker: CallTracker::new(Arc::clone(self), sequence),
            rules_at_start,
        }
    }
}
