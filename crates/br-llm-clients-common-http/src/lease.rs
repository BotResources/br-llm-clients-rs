use std::sync::Arc;

use crate::h2state::Permit;
use crate::pool::{Pool, PoolKey};
use crate::transport::H1Conn;

pub(crate) enum Lease {
    Shared(Permit),
    Exclusive {
        conn: H1Conn,
        pool: Arc<Pool>,
        key: PoolKey,
    },
}

impl Lease {
    pub(crate) fn finish_clean(self) {
        match self {
            Self::Exclusive { conn, pool, key } => pool.release_h1(&key, conn),
            Self::Shared(permit) => drop(permit),
        }
    }
}
