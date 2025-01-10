use std::cmp;
use std::time::Duration;

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

use crate::num::AsF64;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Into, From)]
#[cfg_attr(feature = "fake", derive(fake::Dummy))]
pub struct Seconds(
    #[cfg_attr(
        feature = "fake",
        dummy(expr = "Duration::from_secs_f64(fake::Fake::fake(&(0.0..=69420.0)))")
    )]
    pub Duration,
);

impl AsF64 for Seconds {
    fn as_f64(&self) -> f64 {
        self.0.as_secs_f64()
    }
}

impl From<Seconds> for f32 {
    fn from(secs: Seconds) -> Self {
        secs.0.as_secs_f32()
    }
}

impl From<f32> for Seconds {
    fn from(secs: f32) -> Self {
        Self(Duration::from_secs_f32(secs))
    }
}

impl From<Seconds> for f64 {
    fn from(secs: Seconds) -> Self {
        secs.0.as_secs_f64()
    }
}

impl From<f64> for Seconds {
    fn from(secs: f64) -> Self {
        Self(Duration::from_secs_f64(secs))
    }
}

impl PartialEq<f32> for Seconds {
    fn eq(&self, other: &f32) -> bool {
        self.0.as_secs_f32() == *other
    }
}

impl PartialEq<Seconds> for f32 {
    fn eq(&self, other: &Seconds) -> bool {
        *self == other.0.as_secs_f32()
    }
}

impl PartialEq<f64> for Seconds {
    fn eq(&self, other: &f64) -> bool {
        self.0.as_secs_f64() == *other
    }
}

impl PartialEq<Seconds> for f64 {
    fn eq(&self, other: &Seconds) -> bool {
        *self == other.0.as_secs_f64()
    }
}

impl PartialOrd<f32> for Seconds {
    fn partial_cmp(&self, other: &f32) -> Option<cmp::Ordering> {
        self.0.as_secs_f32().partial_cmp(other)
    }
}

impl PartialOrd<Seconds> for f32 {
    fn partial_cmp(&self, other: &Seconds) -> Option<cmp::Ordering> {
        self.partial_cmp(&other.0.as_secs_f32())
    }
}

impl PartialOrd<f64> for Seconds {
    fn partial_cmp(&self, other: &f64) -> Option<cmp::Ordering> {
        self.0.as_secs_f64().partial_cmp(other)
    }
}

impl PartialOrd<Seconds> for f64 {
    fn partial_cmp(&self, other: &Seconds) -> Option<cmp::Ordering> {
        self.partial_cmp(&other.0.as_secs_f64())
    }
}

impl Serialize for Seconds {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_secs_f64().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Seconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        f64::deserialize(deserializer)
            .map(Duration::from_secs_f64)
            .map(Self)
    }
}

crate::database::impl_traits!(Seconds as f64 => {
    fn encode(self, out: f64) {
        out = self.0.as_secs_f64();
    }

    fn decode(value: f64) -> Result<Self, BoxError> {
        Ok(Self(Duration::from_secs_f64(value)))
    }
});
