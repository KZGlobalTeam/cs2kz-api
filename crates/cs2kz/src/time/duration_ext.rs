use std::ops;

pub trait DurationExt: Sized + ops::Mul<u32, Output = Self> {
    fn day() -> Self;

    fn week() -> Self {
        Self::day() * 7
    }

    fn month() -> Self {
        Self::day() * 30
    }

    fn year() -> Self {
        Self::day() * 365
    }
}

impl DurationExt for std::time::Duration {
    fn day() -> Self {
        Self::from_secs(60 * 60 * 24)
    }
}

impl DurationExt for time::Duration {
    fn day() -> Self {
        Self::seconds(60 * 60 * 24)
    }
}
