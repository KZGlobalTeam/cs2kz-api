use std::{fmt, ops};

use time::OffsetDateTime;

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    From,
    serde::Serialize,
    serde::Deserialize,
    sqlx::Type
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct Timestamp(#[serde(with = "time::serde::rfc3339")] OffsetDateTime);

#[derive(Debug, Display, Error, From)]
#[display("{_0}")]
pub struct OutOfRange(time::error::ComponentRange);

impl Timestamp {
    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    #[tracing::instrument(level = "trace", err)]
    pub fn from_unix_ms(unix_ms: u64) -> Result<Self, OutOfRange> {
        OffsetDateTime::from_unix_timestamp_nanos(unix_ms.into())
            .map(Self)
            .map_err(OutOfRange)
    }

    pub fn to_unix_ms(self) -> u64 {
        (self.0.unix_timestamp_nanos() / 1_000_000)
            .try_into()
            .expect("should be positive")
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .format_into(
                &mut crate::fmt::IoCompat(fmt),
                &time::format_description::well_known::Rfc3339,
            )
            .expect("failed to format timestamp");

        Ok(())
    }
}

impl ops::Add<time::Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, duration: time::Duration) -> Self::Output {
        Timestamp(self.0 + duration)
    }
}

impl ops::Add<Timestamp> for time::Duration {
    type Output = Timestamp;

    fn add(self, timestamp: Timestamp) -> Self::Output {
        timestamp + self
    }
}

impl ops::Add<std::time::Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, duration: std::time::Duration) -> Self::Output {
        Timestamp(self.0 + duration)
    }
}

impl ops::Add<Timestamp> for std::time::Duration {
    type Output = Timestamp;

    fn add(self, timestamp: Timestamp) -> Self::Output {
        timestamp + self
    }
}
