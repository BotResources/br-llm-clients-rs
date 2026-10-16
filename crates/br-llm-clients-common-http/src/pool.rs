use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

use crate::claim::{Claim, Landing};
use crate::dialer::Dialer;
use crate::endpoint::Endpoint;
use crate::host::{Host, Idle, Proto};
#[cfg(test)]
use crate::pool_hooks::Hooks;
use crate::settled::Settled;
use crate::transport::{Conn, H1Conn, LiveGuard, Signal, bump};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PoolKey {
    https: bool,
    host: String,
    port: u16,
}

impl PoolKey {
    pub(crate) fn of(endpoint: &Endpoint) -> Self {
        Self {
            https: endpoint.is_https(),
            host: endpoint.dns_name().to_ascii_lowercase(),
            port: endpoint.effective_port(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct HostHandle {
    pub(crate) signal: Signal,
    pub(crate) pool: Weak<Pool>,
    pub(crate) key: PoolKey,
}

impl HostHandle {
    pub(crate) fn sweep(&self) {
        if let Some(pool) = self.pool.upgrade() {
            pool.forget_if_unused(&self.key);
        }
    }
}

pub(crate) enum WaitKind {
    Dial(Arc<Landing>),
    Stream(Option<Claim>),
}

pub(crate) struct Wait {
    pub(crate) changed: watch::Receiver<u64>,
    pub(crate) kind: WaitKind,
    pub(crate) guard: LiveGuard,
}

pub(crate) enum Plan {
    Reuse(Conn),
    TryIdle(H1Conn),
    Dial(Dialer),
    Wait(Wait),
    Settled(Settled),
}

pub(crate) struct Pool {
    hosts: Mutex<HashMap<PoolKey, Host>>,
    idle_limit: Duration,
    #[cfg(test)]
    pub(crate) hooks: Hooks,
}

impl Pool {
    pub(crate) fn new(idle_limit: Duration) -> Self {
        Self {
            hosts: Mutex::new(HashMap::new()),
            idle_limit,
            #[cfg(test)]
            hooks: Hooks::default(),
        }
    }

    fn reusable(&self, host: &Host) -> Option<Plan> {
        for entry in &host.h2 {
            if let Some(permit) = entry.state.try_acquire() {
                return Some(Plan::Reuse(Conn::H2 {
                    sender: entry.sender.clone(),
                    permit,
                }));
            }
        }
        #[cfg(test)]
        self.hooks.run_after_scan();
        None
    }

    #[cfg(test)]
    pub(crate) fn h2_connections(&self) -> usize {
        self.lock().values().map(|host| host.h2.len()).sum()
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, HashMap<PoolKey, Host>> {
        self.hosts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn plan(self: &Arc<Self>, key: &PoolKey, held: Option<&Claim>) -> Plan {
        let mut hosts = self.lock();
        hosts.retain(|other, host| other == key || host.in_use());
        let host = hosts.entry(key.clone()).or_insert_with(Host::new);
        let changes = host.signal.subscribe();
        let room = host.reclaim_h2_room();
        host.h1.retain(|idle| {
            !idle.conn.sender.is_closed() && idle.since.elapsed() <= self.idle_limit
        });
        if !key.https {
            host.proto = Some(Proto::H1);
        } else if host.h2.is_empty() && host.live_h1.load(Ordering::SeqCst) == 0 && !host.dialing {
            host.proto = None;
        }
        #[cfg(test)]
        self.hooks.run_before_scan();
        let unannounced = host
            .h2
            .iter()
            .find(|entry| entry.state.limit_unknown())
            .map(|entry| Claim::Conn(Arc::clone(&entry.state)));
        #[cfg(test)]
        self.hooks.run_after_unannounced();
        if let Some(settled) = held.and_then(Claim::settled) {
            return Plan::Settled(settled);
        }
        if let Some(reuse) = self.reusable(host) {
            return reuse;
        }
        if let Some(idle) = host.h1.pop() {
            return Plan::TryIdle(idle.conn);
        }
        let landing_claim = || Claim::Landing(Arc::clone(&host.landing));
        let wait = |kind| {
            Plan::Wait(Wait {
                changed: changes,
                kind,
                guard: LiveGuard::new(&host.waiting),
            })
        };
        if unannounced.is_some() {
            return wait(WaitKind::Stream(unannounced));
        }
        match host.proto {
            Some(Proto::H1) => Plan::Dial(self.dialer(key, host, false)),
            Some(Proto::H2) if room && !host.dialing => {
                host.begin_gated_dial();
                Plan::Dial(self.dialer(key, host, true))
            }
            Some(Proto::H2) => wait(WaitKind::Stream(host.dialing.then(landing_claim))),
            None if host.dialing => wait(WaitKind::Dial(Arc::clone(&host.landing))),
            None => {
                host.begin_gated_dial();
                Plan::Dial(self.dialer(key, host, true))
            }
        }
    }

    fn dialer(self: &Arc<Self>, key: &PoolKey, host: &Host, gated: bool) -> Dialer {
        Dialer::on(
            host,
            Arc::downgrade(self),
            key.clone(),
            gated.then(|| Arc::clone(&host.landing)),
        )
    }

    pub(crate) fn release_h1(&self, key: &PoolKey, conn: H1Conn) {
        conn.idle.arm();
        let mut hosts = self.lock();
        let host = hosts.entry(key.clone()).or_insert_with(Host::new);
        host.h1.retain(|idle| !idle.conn.sender.is_closed());
        host.h1.push(Idle {
            conn,
            since: Instant::now(),
        });
        if host.trim_to_cap() {
            bump(&host.signal);
        }
    }

    pub(crate) fn forget_if_unused(&self, key: &PoolKey) {
        let mut hosts = self.lock();
        let Some(host) = hosts.get_mut(key) else {
            return;
        };
        host.drop_ended_h2();
        host.h1.retain(|idle| !idle.conn.sender.is_closed());
        if !host.in_use() {
            hosts.remove(key);
        }
    }

    pub(crate) fn trim_idle(&self, key: &PoolKey) {
        let mut hosts = self.lock();
        let Some(host) = hosts.get_mut(key) else {
            return;
        };
        if host.trim_to_cap() {
            bump(&host.signal);
        }
    }
}
