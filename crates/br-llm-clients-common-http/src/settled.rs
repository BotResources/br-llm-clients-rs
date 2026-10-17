use crate::dialer::DialFailure;

pub(crate) enum Settled {
    Spent,
    Broken(String),
    Failed(DialFailure),
}
