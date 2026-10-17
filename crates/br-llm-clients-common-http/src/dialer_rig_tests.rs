use std::sync::Arc;

use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;

use crate::acquire_tests::Rig;
use crate::claim::Claim;
use crate::dialer::Dialer;
use crate::host::{Host, Proto};
use crate::idle::{IdleClock, IdleWatch};
use crate::payload::Payload;
use crate::pool::Plan;
use crate::transport::{H1Conn, LiveGuard};
use crate::tuning::Tuning;

pub(crate) async fn h1_connection_watched() -> (H1Conn, tokio::io::DuplexStream, IdleWatch) {
    let (client, server) = tokio::io::duplex(4096);
    let (sender, connection) = http1::Builder::new()
        .handshake::<_, Payload>(TokioIo::new(client))
        .await
        .expect("handshake");
    let driver = tokio::spawn(async move {
        let _ = connection.await;
    });
    let (idle, watch) = IdleClock::new(Tuning::FIXED.idle_limit);
    (
        H1Conn::new(sender, idle, driver.abort_handle()),
        server,
        watch,
    )
}

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

    pub(crate) async fn h1_connection(&self) -> (H1Conn, tokio::io::DuplexStream) {
        let (conn, server, _watch) = h1_connection_watched().await;
        (conn, server)
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
