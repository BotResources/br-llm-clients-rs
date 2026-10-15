use std::time::Duration;

use crate::acquire::READY_GRACE;
use crate::dial::STAGGER;
use crate::limits::{MAX_ATTEMPTS, MAX_HEAD_BYTES, MAX_HEADER_LINES};
use crate::pool::{MAX_H2_PER_HOST, MAX_IDLE_PER_HOST};
use crate::tuning::Tuning;

#[test]
fn given_the_fixed_numbers_of_the_intent_then_they_have_exactly_these_values() {
    assert_eq!(Tuning::FIXED.idle_limit, Duration::from_secs(30));
    assert_eq!(Tuning::FIXED.ping_interval, Duration::from_secs(30));
    assert_eq!(Tuning::FIXED.ping_timeout, Duration::from_secs(10));
    assert_eq!(STAGGER, Duration::from_millis(250));
    assert_eq!(READY_GRACE, Duration::from_millis(250));
    assert_eq!(MAX_IDLE_PER_HOST, 8);
    assert_eq!(MAX_H2_PER_HOST, 16);
    assert_eq!(MAX_ATTEMPTS, 3);
    assert_eq!(MAX_HEADER_LINES, 1024);
    assert_eq!(MAX_HEAD_BYTES, 64 * 1024);
}
