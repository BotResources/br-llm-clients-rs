use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

use crate::dialer::{DialFailure, Dialer};
use crate::endpoint::Endpoint;
use crate::h2state::H2State;
use crate::transport::{Conn, H1Conn, H2Sender, Signal};

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

#[derive(Clone)]
pub(crate) struct HostHandle {
    pub(crate) signal: Signal,
    pub(crate) pool: Weak<Pool>,
    pub(crate) key: PoolKey,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Proto {
    H1,
    H2,
}

struct Idle {
    conn: H1Conn,
    since: Instant,
}

pub(crate) struct H2Entry {
    pub(crate) sender: H2Sender,
    pub(crate) state: Arc<H2State>,
}

struct Recorded {
    seq: u64,
    failure: DialFailure,
}

pub(crate) struct Host {
    pub(crate) proto: Option<Proto>,
    pub(crate) dialing: bool,
    pub(crate) h2: Vec<H2Entry>,
    h1: Vec<Idle>,
    pub(crate) live_h1: Arc<AtomicUsize>,
    pub(crate) signal: Signal,
    failure: Option<Recorded>,
    failures: u64,
}

impl Host {
    pub(crate) fn new() -> Self {
        Self {
            proto: None,
            dialing: false,
            h2: Vec::new(),
            h1: Vec::new(),
            live_h1: Arc::new(AtomicUsize::new(0)),
            signal: Arc::new(watch::channel(0u64).0),
            failure: None,
            failures: 0,
        }
    }

    fn in_use(&self) -> bool {
        self.dialing
            || self.failure.is_some()
            || !self.h2.is_empty()
            || !self.h1.is_empty()
            || self.live_h1.load(Ordering::SeqCst) > 0
    }

    pub(crate) fn record_failure(&mut self, failure: DialFailure) {
        self.failures = self.failures.saturating_add(1);
        self.failure = Some(Recorded {
            seq: self.failures,
            failure,
        });
    }

    pub(crate) fn clear_failure(&mut self) {
        self.failure = None;
    }

    fn idle_total(&self) -> usize {
        let idle_h2 = self.h2.iter().filter(|e| e.state.is_idle()).count();
        self.h1.len().saturating_add(idle_h2)
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
    Wait(watch::Receiver<u64>, WaitKind, u64),
    Failed(DialFailure),
}

pub(crate) struct Pool {
    hosts: Mutex<HashMap<PoolKey, Host>>,
    idle_limit: Duration,
}

impl Pool {
    pub(crate) fn new(idle_limit: Duration) -> Self {
        Self {
            hosts: Mutex::new(HashMap::new()),
            idle_limit,
        }
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, HashMap<PoolKey, Host>> {
        self.hosts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn plan(self: &Arc<Self>, key: &PoolKey, waited_from: Option<u64>) -> Plan {
        let mut hosts = self.lock();
        hosts.retain(|other, host| other == key || host.in_use());
        let host = hosts.entry(key.clone()).or_insert_with(Host::new);
        let changes = host.signal.subscribe();
        host.h2
            .retain(|entry| !entry.state.is_retired() && !entry.sender.is_closed());
        host.h1.retain(|idle| {
            !idle.conn.sender.is_closed() && idle.since.elapsed() <= self.idle_limit
        });
        if !key.https {
            host.proto = Some(Proto::H1);
        } else if host.h2.is_empty() && host.live_h1.load(Ordering::SeqCst) == 0 && !host.dialing {
            host.proto = None;
        }
        let limit_unknown = host.h2.iter().any(|entry| entry.state.limit_unknown());
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
        let seq = host.failure.as_ref().map_or(0, |recorded| recorded.seq);
        if limit_unknown {
            return Plan::Wait(changes, WaitKind::Stream, seq);
        }
        match host.proto {
            Some(Proto::H1) => Plan::Dial(self.dialer(key, host, false)),
            Some(Proto::H2) if host.h2.len() < MAX_H2_PER_HOST && !host.dialing => {
                host.dialing = true;
                Plan::Dial(self.dialer(key, host, true))
            }
            Some(Proto::H2) => Plan::Wait(changes, WaitKind::Stream, seq),
            None if host.dialing => Plan::Wait(changes, WaitKind::Dial, seq),
            None => match (&host.failure, waited_from) {
                (Some(recorded), Some(from)) if recorded.seq > from => {
                    Plan::Failed(recorded.failure.clone())
                }
                _ => {
                    host.dialing = true;
                    Plan::Dial(self.dialer(key, host, true))
                }
            },
        }
    }

    fn dialer(self: &Arc<Self>, key: &PoolKey, host: &Host, gated: bool) -> Dialer {
        Dialer::new(
            HostHandle {
                signal: Arc::clone(&host.signal),
                pool: Arc::downgrade(self),
                key: key.clone(),
            },
            gated,
        )
    }

    pub(crate) fn release_h1(&self, key: &PoolKey, conn: H1Conn) {
        conn.idle.arm();
        let mut hosts = self.lock();
        let host = hosts.entry(key.clone()).or_insert_with(Host::new);
        host.h1.retain(|idle| !idle.conn.sender.is_closed());
        while host.idle_total() >= MAX_IDLE_PER_HOST && !host.h1.is_empty() {
            host.h1.remove(0);
        }
        host.h1.push(Idle {
            conn,
            since: Instant::now(),
        });
    }

    pub(crate) fn trim_idle(&self, key: &PoolKey) {
        let mut hosts = self.lock();
        let Some(host) = hosts.get_mut(key) else {
            return;
        };
        while host.idle_total() > MAX_IDLE_PER_HOST {
            let oldest_idle = host
                .h2
                .iter()
                .position(|entry| entry.state.is_idle() && !entry.state.is_retired());
            match oldest_idle {
                Some(index) => {
                    let entry = host.h2.remove(index);
                    entry.state.retire();
                }
                None => break,
            }
        }
    }
}
