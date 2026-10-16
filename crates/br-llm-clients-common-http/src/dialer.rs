use std::sync::{Arc, Weak};

use crate::claim::Landing;
use crate::connect::Opened;
use crate::host::{H2Entry, Host, Proto};
use crate::pool::{HostHandle, Pool, PoolKey};
use crate::transport::{Conn, LiveGuard, bump};

#[derive(Clone)]
pub(crate) enum DialFailure {
    Connect(String),
    Tls(String),
    Timeout,
}

impl DialFailure {
    pub(crate) fn lost() -> Self {
        Self::Connect("the connection attempt ended without a result".to_owned())
    }
}

pub(crate) enum Registered {
    Connected(Conn),
    Rejected,
    PoolGone,
}

impl Registered {
    pub(crate) fn into_conn(self) -> Option<Conn> {
        match self {
            Self::Connected(conn) => Some(conn),
            Self::Rejected | Self::PoolGone => None,
        }
    }
}

pub(crate) struct Dialer {
    pub(crate) host: HostHandle,
    landing: Option<Arc<Landing>>,
    done: bool,
    _pin: LiveGuard,
}

impl Dialer {
    pub(crate) fn on(
        host: &Host,
        pool: Weak<Pool>,
        key: PoolKey,
        landing: Option<Arc<Landing>>,
    ) -> Self {
        Self {
            host: HostHandle {
                signal: Arc::clone(&host.signal),
                pool,
                key,
            },
            landing,
            done: false,
            _pin: host.pin_for_dial(),
        }
    }

    fn gated(&self) -> bool {
        self.landing.is_some()
    }

    pub(crate) fn register(mut self, opened: Opened) -> Registered {
        let Some(pool) = self.host.pool.upgrade() else {
            return Registered::PoolGone;
        };
        #[cfg(test)]
        pool.hooks.run_before_register();
        let mut hosts = pool.lock();
        let Some(host) = hosts.get_mut(&self.host.key) else {
            return Registered::PoolGone;
        };
        self.done = true;
        let conn = match opened {
            Opened::H1(mut conn) => {
                host.proto = Some(Proto::H1);
                conn.count_as_live(&host.live_h1);
                Registered::Connected(Conn::H1(conn))
            }
            Opened::H2 { sender, state } => {
                host.proto = Some(Proto::H2);
                if !host.has_h2_room() {
                    state.retire();
                    if let Some(landing) = &self.landing {
                        landing.land(&state);
                    }
                    Registered::Rejected
                } else {
                    let permit = state.force_acquire();
                    if let Some(landing) = &self.landing {
                        landing.land(&state);
                    }
                    host.h2.push(H2Entry {
                        sender: sender.clone(),
                        state,
                    });
                    Registered::Connected(Conn::H2 { sender, permit })
                }
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
        if std::thread::panicking()
            && let Some(landing) = &self.landing
        {
            landing.fail(DialFailure::lost());
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
