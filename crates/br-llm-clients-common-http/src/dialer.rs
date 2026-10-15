use crate::connect::Opened;
use crate::pool::{Host, HostHandle, Proto};
use crate::transport::{Conn, bump};

#[derive(Clone)]
pub(crate) enum DialFailure {
    Connect(String),
    Tls(String),
    Timeout,
}

pub(crate) struct Dialer {
    pub(crate) host: HostHandle,
    gated: bool,
    done: bool,
}

impl Dialer {
    pub(crate) fn new(host: HostHandle, gated: bool) -> Self {
        Self {
            host,
            gated,
            done: false,
        }
    }

    pub(crate) fn register(mut self, opened: Opened) -> Option<Conn> {
        let pool = self.host.pool.upgrade()?;
        let mut hosts = pool.lock();
        let host = hosts.entry(self.host.key.clone()).or_insert_with(Host::new);
        self.done = true;
        let conn = match opened {
            Opened::H1(mut conn) => {
                host.proto = Some(Proto::H1);
                conn.count_as_live(&host.live_h1);
                Some(Conn::H1(conn))
            }
            Opened::H2 { sender, state } => {
                host.proto = Some(Proto::H2);
                let permit = state.force_acquire();
                host.h2.push(crate::pool::H2Entry {
                    sender: sender.clone(),
                    state,
                });
                Some(Conn::H2 { sender, permit })
            }
        };
        host.clear_failure();
        if self.gated {
            host.dialing = false;
        }
        bump(&host.signal);
        conn
    }

    pub(crate) fn fail(mut self, failure: DialFailure) {
        self.done = true;
        if !self.gated {
            return;
        }
        if let Some(pool) = self.host.pool.upgrade() {
            let mut hosts = pool.lock();
            if let Some(host) = hosts.get_mut(&self.host.key) {
                host.dialing = false;
                host.record_failure(failure);
                bump(&host.signal);
            }
        }
    }
}

impl Drop for Dialer {
    fn drop(&mut self) {
        if !self.gated || self.done {
            return;
        }
        if let Some(pool) = self.host.pool.upgrade() {
            let mut hosts = pool.lock();
            if let Some(host) = hosts.get_mut(&self.host.key) {
                host.dialing = false;
                bump(&host.signal);
            }
        }
    }
}
