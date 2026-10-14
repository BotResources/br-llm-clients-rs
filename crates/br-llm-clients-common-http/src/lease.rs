use std::sync::Arc;

use crate::pool::{Pool, PoolKey};
use crate::transport::H1Conn;

pub(crate) enum Lease {
    Shared,
    Exclusive {
        conn: H1Conn,
        pool: Arc<Pool>,
        key: PoolKey,
    },
}

impl Lease {
    pub(crate) fn finish_clean(self) {
        if let Self::Exclusive { conn, pool, key } = self {
            pool.release_h1(&key, conn);
        }
    }
}
