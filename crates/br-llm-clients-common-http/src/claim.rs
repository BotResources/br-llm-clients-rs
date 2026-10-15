use std::sync::{Arc, OnceLock};

use crate::h2state::{H2State, Standing};

pub(crate) struct Landing {
    state: OnceLock<Arc<H2State>>,
}

impl Landing {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: OnceLock::new(),
        })
    }

    pub(crate) fn land(&self, state: &Arc<H2State>) {
        let _ = self.state.set(Arc::clone(state));
    }

    pub(crate) fn landed(&self) -> Option<&Arc<H2State>> {
        self.state.get()
    }
}

pub(crate) enum Claim {
    Conn(Arc<H2State>),
    Landing(Arc<Landing>),
}

impl Claim {
    pub(crate) fn standing(&self) -> Standing {
        match self {
            Self::Conn(state) => state.standing(),
            Self::Landing(landing) => landing.landed().map_or(Standing::Open, |s| s.standing()),
        }
    }
}
