use std::time::{Duration, SystemTime};

pub(crate) fn parse(value: &[u8], now: SystemTime) -> Option<Duration> {
    let text = std::str::from_utf8(value).ok()?.trim_matches([' ', '\t']);
    if text.is_empty() {
        return None;
    }
    if text.bytes().all(|b| b.is_ascii_digit()) {
        return Some(seconds(text));
    }
    let date = httpdate::parse_http_date(text).ok()?;
    Some(date.duration_since(now).unwrap_or(Duration::ZERO))
}

fn seconds(digits: &str) -> Duration {
    let mut total: u64 = 0;
    for byte in digits.bytes() {
        let digit = u64::from(byte.saturating_sub(b'0'));
        match total.checked_mul(10).and_then(|t| t.checked_add(digit)) {
            Some(next) => total = next,
            None => return Duration::MAX,
        }
    }
    Duration::from_secs(total)
}
