use std::time::{Duration, SystemTime};

pub(crate) fn parse(value: &[u8], now: SystemTime) -> Option<Duration> {
    let text = trimmed(value)?;
    if text.is_empty() {
        return None;
    }
    if all_digits(text) {
        return Some(seconds(text));
    }
    let date = httpdate::parse_http_date(text).ok()?;
    Some(date.duration_since(now).unwrap_or(Duration::ZERO))
}

pub(crate) fn parse_millis(value: &[u8]) -> Option<Duration> {
    let text = trimmed(value)?;
    if text.is_empty() || !all_digits(text) {
        return None;
    }
    Some(milliseconds(text))
}

fn trimmed(value: &[u8]) -> Option<&str> {
    Some(std::str::from_utf8(value).ok()?.trim_matches([' ', '\t']))
}

fn all_digits(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii_digit())
}

fn count(digits: &str) -> Option<u128> {
    let mut total: u128 = 0;
    for byte in digits.bytes() {
        let digit = u128::from(byte.saturating_sub(b'0'));
        total = total.checked_mul(10)?.checked_add(digit)?;
    }
    Some(total)
}

fn seconds(digits: &str) -> Duration {
    count(digits)
        .and_then(|total| u64::try_from(total).ok())
        .map_or(Duration::MAX, Duration::from_secs)
}

fn milliseconds(digits: &str) -> Duration {
    let Some(total) = count(digits) else {
        return Duration::MAX;
    };
    let Ok(whole_seconds) = u64::try_from(total / 1000) else {
        return Duration::MAX;
    };
    Duration::new(whole_seconds, rest_millis(digits).saturating_mul(1_000_000))
}

fn rest_millis(digits: &str) -> u32 {
    digits.bytes().fold(0, |rest, byte| {
        let digit = u32::from(byte.saturating_sub(b'0'));
        rest.saturating_mul(10).saturating_add(digit) % 1000
    })
}
