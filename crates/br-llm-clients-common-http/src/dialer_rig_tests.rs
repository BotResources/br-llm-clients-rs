use std::sync::Arc;

use crate::acquire_tests::Rig;
use crate::claim::Claim;
use crate::dialer::Dialer;
use crate::host::{Host, Proto};
use crate::pool::Plan;
use crate::transport::LiveGuard;

impl Rig {
    pub(crate) fn ungated_dialer(&self) -> Dialer {
        let mut hosts = self.shared.pool.lock();
        let host = hosts.entry(self.key.clone()).or_insert_with(Host::new);
        Dialer::on(
            host,
            Arc::downgrade(&self.shared.pool),
            self.key.clone(),
            None,
        )
    }

    pub(crate) fn gated_dialer(&self) -> (Dialer, Claim) {
        let mut hosts = self.shared.pool.lock();
        let host = hosts.entry(self.key.clone()).or_insert_with(Host::new);
        host.begin_gated_dial();
        let landing = Arc::clone(&host.landing);
        let dialer = Dialer::on(
            host,
            Arc::downgrade(&self.shared.pool),
            self.key.clone(),
            Some(Arc::clone(&landing)),
        );
        (dialer, Claim::Landing(landing))
    }

    pub(crate) fn http1_connection_alive(&self) -> LiveGuard {
        let mut hosts = self.shared.pool.lock();
        let host = hosts.entry(self.key.clone()).or_insert_with(Host::new);
        host.proto = Some(Proto::H1);
        LiveGuard::new(&host.live_h1)
    }

    pub(crate) fn ungated_dial_planned_while_an_http1_connection_is_alive(
        &self,
    ) -> (LiveGuard, Dialer) {
        let alive = self.http1_connection_alive();
        let Plan::Dial(dialer) = self.shared.pool.plan(&self.key, None) else {
            panic!("the send dials");
        };
        assert!(!self.dialing(), "the dial must not hold the gate");
        (alive, dialer)
    }
}
