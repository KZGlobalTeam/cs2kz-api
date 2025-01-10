use std::{cmp, fmt};

use futures_util::stream::{MapOk, Stream, TryStreamExt};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Display, Clone, Copy, Into, Serialize)]
#[display("{value}")]
#[serde(transparent)]
pub struct Limit<const MAX: u64, const DEFAULT: u64> {
    value: u64,
}

impl<const MAX: u64, const DEFAULT: u64> Limit<MAX, DEFAULT> {
    pub const MAX: Self = Self { value: MAX };

    pub const fn new(value: u64) -> Self {
        const {
            assert!(DEFAULT <= MAX, "`DEFAULT` for `Limit` cannot be greater than `MAX`");
        }

        Self { value: if value > MAX { MAX } else { value } }
    }

    pub const fn value(&self) -> u64 {
        self.value
    }
}

impl<const MAX: u64, const DEFAULT: u64> Default for Limit<MAX, DEFAULT> {
    fn default() -> Self {
        Self { value: DEFAULT }
    }
}

impl<'de, const MAX: u64, const DEFAULT: u64> Deserialize<'de> for Limit<MAX, DEFAULT> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<u64>::deserialize(deserializer).map(|value| {
            value.map_or_else(Self::default, |value| Self { value: cmp::min(value, MAX) })
        })
    }
}

#[derive(Debug, Display, Default, Clone, Copy, From, Into, Serialize, Deserialize)]
#[display("{value}")]
#[serde(default, transparent)]
pub struct Offset {
    value: i64,
}

impl Offset {
    pub const fn new(value: i64) -> Self {
        Self { value }
    }

    pub const fn value(&self) -> i64 {
        self.value
    }
}

#[derive(Serialize)]
pub struct Paginated<T> {
    total: u64,
    values: T,
}

impl<T> Paginated<T> {
    pub fn new(total: u64, values: T) -> Self {
        Self { total, values }
    }

    pub fn into_inner(self) -> T {
        self.values
    }
}

impl<T> Paginated<Vec<T>> {
    pub fn map_values<U>(self, f: impl FnMut(T) -> U) -> Paginated<Vec<U>> {
        Paginated {
            total: self.total,
            values: self.values.into_iter().map(f).collect(),
        }
    }
}

impl<S, T, E> Paginated<S>
where
    S: Stream<Item = Result<T, E>>,
    E: std::error::Error,
{
    /// Transforms the `Ok` values of the underlying stream.
    pub fn map<F, U>(self, f: F) -> Paginated<MapOk<S, F>>
    where
        F: FnMut(T) -> U,
    {
        Paginated {
            total: self.total,
            values: self.values.map_ok(f),
        }
    }

    /// Transforms the `Ok` values of the underlying stream by calling [`Into::into()`].
    pub fn map_into<U>(self) -> Paginated<MapOk<S, impl Fn(T) -> U>>
    where
        T: Into<U>,
    {
        self.map(Into::into)
    }

    /// Collects the stream into `C` or short-circuits with the first error that is encountered.
    #[tracing::instrument(level = "trace", err(level = "debug"))]
    pub async fn collect<C: Default + Extend<T>>(self) -> Result<Paginated<C>, E> {
        Ok(Paginated {
            total: self.total,
            values: self.values.try_collect().await?,
        })
    }
}

impl<T> fmt::Debug for Paginated<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Paginated")
            .field("total", &self.total)
            .finish()
    }
}
