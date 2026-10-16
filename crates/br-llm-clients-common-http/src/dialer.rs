use std::sync::Arc;

use crate::claim::Landing;
use crate::connect::Opened;
use crate::host::{H2Entry, Host, Proto};
use crate::pool::HostHandle;
use crate::transport::{Conn, bump};

#[derive(Clone)]
pub(crate) enum DialFailure {
    Connect(String),
    Tls(String),
    Timeout,
}

pub(crate) struct Dialer {
    pub(crate) host: HostHandle,
    landing: Option<Arc<Landing>>,
    done: bool,
}

impl Dialer {
    pub(crate) fn new(host: HostHandle, landing: Option<Arc<Landing>>) -> Self {
        Self {
            host,
            landing,
            done: false,
        }
    }

    fn gated(&self) -> bool {
        self.landing.is_some()
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
                if let Some(landing) = &self.landing {
                    landing.land(&state);
                }
                host.h2.push(H2Entry {
                    sender: sender.clone(),
                    state,
                });
                Some(Conn::H2 { sender, permit })
            }
        };
        if self.gated() {
            host.dialing = false;
        }
        bump(&host.signal);
        conn
    }

    pub(crate) fn fail(mut self, failure: DialFailure) {
        self.done = true;
        let Some(landing) = &self.landing else {
            return;
        };
        landing.fail(failure);
        if let Some(pool) = self.host.pool.upgrade() {
            let mut hosts = pool.lock();
            if let Some(host) = hosts.get_mut(&self.host.key) {
                host.dialing = false;
                bump(&host.signal);
            }
        }
    }
}

impl Drop for Dialer {
    fn drop(&mut self) {
        if !self.gated() || self.done {
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
