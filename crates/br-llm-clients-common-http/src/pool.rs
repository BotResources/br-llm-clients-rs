use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

use crate::connect::Opened;
use crate::endpoint::Endpoint;
use crate::transport::{Conn, H1Conn, H2Sender, H2State, Signal, bump};

pub(crate) const MAX_IDLE_PER_HOST: usize = 8;
pub(crate) const MAX_H2_PER_HOST: usize = 16;

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Proto {
    H1,
    H2,
}

struct Idle {
    conn: H1Conn,
    since: Instant,
}

struct H2Entry {
    sender: H2Sender,
    state: Arc<H2State>,
}

struct Host {
    proto: Option<Proto>,
    dialing: bool,
    h2: Vec<H2Entry>,
    h1: Vec<Idle>,
    live_h1: Arc<AtomicUsize>,
    signal: Signal,
}

impl Host {
    fn new() -> Self {
        Self {
            proto: None,
            dialing: false,
            h2: Vec::new(),
            h1: Vec::new(),
            live_h1: Arc::new(AtomicUsize::new(0)),
            signal: Arc::new(watch::channel(0u64).0),
        }
    }
}

pub(crate) enum WaitKind {
    Dial,
    Stream,
}

pub(crate) enum Plan {
    Reuse(Conn),
    TryIdle(H1Conn),
    Dial(Dialer),
    Wait(watch::Receiver<u64>, WaitKind),
}

pub(crate) struct Pool {
    hosts: Mutex<HashMap<PoolKey, Host>>,
    idle_limit: Duration,
}

pub(crate) struct Dialer {
    pool: Arc<Pool>,
    key: PoolKey,
    gated: bool,
    done: bool,
    pub(crate) signal: Signal,
}

impl Pool {
    pub(crate) fn new(idle_limit: Duration) -> Self {
        Self {
            hosts: Mutex::new(HashMap::new()),
            idle_limit,
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<PoolKey, Host>> {
        self.hosts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn plan(self: &Arc<Self>, key: &PoolKey, fresh: bool) -> Plan {
        let mut hosts = self.lock();
        let host = hosts.entry(key.clone()).or_insert_with(Host::new);
        host.h2.retain(|entry| !entry.sender.is_closed());
        host.h1.retain(|idle| {
            !idle.conn.sender.is_closed() && idle.since.elapsed() <= self.idle_limit
        });
        if !key.https {
            host.proto = Some(Proto::H1);
        } else if host.h2.is_empty() && host.live_h1.load(Ordering::SeqCst) == 0 && !host.dialing {
            host.proto = None;
        }
        if fresh {
            return Plan::Dial(self.dialer(key, host, false));
        }
        for entry in &host.h2 {
            if let Some(permit) = entry.state.try_acquire() {
                return Plan::Reuse(Conn::H2 {
                    sender: entry.sender.clone(),
                    permit,
                });
            }
        }
        if let Some(idle) = host.h1.pop() {
            return Plan::TryIdle(idle.conn);
        }
        match host.proto {
            Some(Proto::H1) => Plan::Dial(self.dialer(key, host, false)),
            Some(Proto::H2) if host.h2.len() < MAX_H2_PER_HOST && !host.dialing => {
                host.dialing = true;
                Plan::Dial(self.dialer(key, host, true))
            }
            Some(Proto::H2) => Plan::Wait(host.signal.subscribe(), WaitKind::Stream),
            None if host.dialing => Plan::Wait(host.signal.subscribe(), WaitKind::Dial),
            None => {
                host.dialing = true;
                Plan::Dial(self.dialer(key, host, true))
            }
        }
    }

    fn dialer(self: &Arc<Self>, key: &PoolKey, host: &Host, gated: bool) -> Dialer {
        Dialer {
            pool: Arc::clone(self),
            key: key.clone(),
            gated,
            done: false,
            signal: Arc::clone(&host.signal),
        }
    }

    pub(crate) fn release_h1(&self, key: &PoolKey, conn: H1Conn) {
        conn.idle.arm();
        let mut hosts = self.lock();
        let host = hosts.entry(key.clone()).or_insert_with(Host::new);
        host.h1.retain(|idle| !idle.conn.sender.is_closed());
        while host.h1.len() >= MAX_IDLE_PER_HOST {
            host.h1.remove(0);
        }
        host.h1.push(Idle {
            conn,
            since: Instant::now(),
        });
    }
}

impl Dialer {
    pub(crate) fn register(mut self, opened: Opened) -> Option<Conn> {
        let mut hosts = self.pool.lock();
        let host = hosts.entry(self.key.clone()).or_insert_with(Host::new);
        self.done = true;
        let conn = match opened {
            Opened::H1(mut conn) => {
                host.proto = Some(Proto::H1);
                conn.count_as_live(&host.live_h1);
                Some(Conn::H1(conn))
            }
            Opened::H2 { sender, state } => {
                host.proto = Some(Proto::H2);
                let permit = state.try_acquire();
                host.h2.push(H2Entry {
                    sender: sender.clone(),
                    state,
                });
                permit.map(|permit| Conn::H2 { sender, permit })
            }
        };
        if self.gated {
            host.dialing = false;
        }
        bump(&host.signal);
        conn
    }
}

impl Drop for Dialer {
    fn drop(&mut self) {
        if self.gated && !self.done {
            let mut hosts = self.pool.lock();
            if let Some(host) = hosts.get_mut(&self.key) {
                host.dialing = false;
                bump(&host.signal);
            }
        }
    }
}
