use std::time::Duration;

use serde::Deserialize;
use serde::de::{self, Deserializer};

const NANOS_PER_SECOND: u32 = 1_000_000_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredDuration {
    secs: u64,
    nanos: u32,
}

pub(crate) fn duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
    let stored = StoredDuration::deserialize(deserializer)?;
    if stored.nanos >= NANOS_PER_SECOND {
        return Err(de::Error::invalid_value(
            de::Unexpected::Unsigned(u64::from(stored.nanos)),
            &"a number of nanoseconds below 1000000000",
        ));
    }
    Ok(Duration::new(stored.secs, stored.nanos))
}

pub(crate) fn present_duration<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    duration(deserializer).map(Some)
}
