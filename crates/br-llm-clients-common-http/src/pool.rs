use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::time::{Instant, timeout};

use crate::endpoint::Endpoint;
use crate::transport::{Conn, H1Conn, H2Sender};

const IDLE_LIMIT: Duration = Duration::from_secs(30);
const READY_GRACE: Duration = Duration::from_millis(250);
const MAX_IDLE_PER_KEY: usize = 8;

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

struct Idle {
    conn: H1Conn,
    since: Instant,
}

#[derive(Default)]
struct Slot {
    h2: Option<H2Sender>,
    h1: Vec<Idle>,
}

#[derive(Default)]
pub(crate) struct Pool {
    slots: Mutex<HashMap<PoolKey, Slot>>,
}

impl Pool {
    fn lock(&self) -> MutexGuard<'_, HashMap<PoolKey, Slot>> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) async fn checkout(&self, key: &PoolKey) -> Option<Conn> {
        loop {
            let idle = {
                let mut slots = self.lock();
                let slot = slots.get_mut(key)?;
                if let Some(sender) = &slot.h2 {
                    if !sender.is_closed() {
                        return Some(Conn::H2(sender.clone()));
                    }
                    slot.h2 = None;
                }
                slot.h1.pop()?
            };
            if idle.since.elapsed() > IDLE_LIMIT {
                continue;
            }
            let mut conn = idle.conn;
            if matches!(timeout(READY_GRACE, conn.sender.ready()).await, Ok(Ok(()))) {
                return Some(Conn::H1(conn));
            }
        }
    }

    pub(crate) fn offer_h2(&self, key: &PoolKey, sender: &H2Sender) {
        let mut slots = self.lock();
        let slot = slots.entry(key.clone()).or_default();
        if slot.h2.as_ref().is_none_or(H2Sender::is_closed) {
            slot.h2 = Some(sender.clone());
        }
    }

    pub(crate) fn release_h1(&self, key: &PoolKey, conn: H1Conn) {
        let mut slots = self.lock();
        let slot = slots.entry(key.clone()).or_default();
        if slot.h1.len() >= MAX_IDLE_PER_KEY {
            slot.h1.remove(0);
        }
        slot.h1.push(Idle {
            conn,
            since: Instant::now(),
        });
    }
}
