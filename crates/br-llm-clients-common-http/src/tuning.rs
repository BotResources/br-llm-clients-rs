use std::time::Duration;

#[derive(Clone, Copy)]
pub(crate) struct Tuning {
    pub(crate) idle_limit: Duration,
    pub(crate) ping_interval: Duration,
    pub(crate) ping_timeout: Duration,
}

impl Tuning {
    pub(crate) const FIXED: Self = Self {
        idle_limit: Duration::from_secs(30),
        ping_interval: Duration::from_secs(30),
        ping_timeout: Duration::from_secs(10),
    };
}
