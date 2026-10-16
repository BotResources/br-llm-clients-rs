use std::sync::{Arc, OnceLock};

use crate::dialer::DialFailure;
use crate::h2state::H2State;

pub(crate) enum Standing {
    Open,
    Spent,
    Broken(String),
    Failed(DialFailure),
}

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

    fn standing(&self) -> Standing {
        if let Some(failure) = self.failure.get() {
            return Standing::Failed(failure.clone());
        }
        self.state
            .get()
            .map_or(Standing::Open, |state| state.standing())
    }
}

pub(crate) enum Claim {
    Conn(Arc<H2State>),
    Landing(Arc<Landing>),
}

impl Claim {
    #[cfg(test)]
    pub(crate) fn into_landing(self) -> Option<Arc<Landing>> {
        match self {
            Self::Landing(landing) => Some(landing),
            Self::Conn(_) => None,
        }
    }

    pub(crate) fn standing(&self) -> Standing {
        match self {
            Self::Conn(state) => state.standing(),
            Self::Landing(landing) => landing.standing(),
        }
    }
}
