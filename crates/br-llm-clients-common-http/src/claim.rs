use std::sync::{Arc, OnceLock};

use crate::dialer::DialFailure;
use crate::h2state::H2State;
use crate::settled::Settled;

pub(crate) struct Landing {
    state: OnceLock<Arc<H2State>>,
    failure: OnceLock<DialFailure>,
}

impl Landing {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: OnceLock::new(),
            failure: OnceLock::new(),
        })
    }

    pub(crate) fn land(&self, state: &Arc<H2State>) {
        let _ = self.state.set(Arc::clone(state));
    }

    #[cfg(test)]
    pub(crate) fn landed(&self) -> Option<&Arc<H2State>> {
        self.state.get()
    }

    pub(crate) fn fail(&self, failure: DialFailure) {
        let _ = self.failure.set(failure);
    }

    fn settled(&self) -> Option<Settled> {
        if let Some(failure) = self.failure.get() {
            return Some(Settled::Failed(failure.clone()));
        }
        self.state.get().and_then(|state| state.settled())
    }
}

pub(crate) enum Claim {
    Conn(Arc<H2State>),
    Landing(Arc<Landing>),
}

impl Claim {
    pub(crate) fn settled(&self) -> Option<Settled> {
        match self {
            Self::Conn(state) => state.settled(),
            Self::Landing(landing) => landing.settled(),
        }
    }
}
