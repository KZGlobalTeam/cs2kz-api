use std::fmt;

use serde::de::{self, Unexpected};

use crate::{CommunityId, SteamId};

#[derive(Debug, Default)]
pub struct VisitEverything {
    visit_u64: VisitU64,
    visit_community_id: VisitCommunityId,
    visit_standard: VisitStandardFormat,
}

#[derive(Debug, Default)]
pub struct VisitU64 {
    _priv: (),
}

#[derive(Debug, Default)]
pub struct VisitCommunityId {
    _priv: (),
}

#[derive(Debug, Default)]
pub struct VisitStandardFormat {
    _priv: (),
}

impl de::Visitor<'_> for VisitEverything {
    type Value = SteamId;

    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "a SteamID")
    }

    fn visit_u32<E: de::Error>(self, value: u32) -> Result<Self::Value, E> {
        self.visit_community_id.visit_u32(value).map(Into::into)
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        if let Ok(value) = u32::try_from(value) {
            return self.visit_community_id.visit_u32(value).map(Into::into);
        }

        self.visit_u64.visit_u64(value)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if let Ok(value) = value.parse::<u64>() {
            self.visit_u64(value)
        } else if let Ok(steam_id) = self.visit_community_id.visit_str::<E>(value) {
            Ok(steam_id.into())
        } else {
            self.visit_standard.visit_str(value)
        }
    }
}

impl de::Visitor<'_> for VisitU64 {
    type Value = SteamId;

    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "a 64-bit SteamID")
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        SteamId::from_u64(value).map_err(de::Error::custom)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        let value = value
            .parse::<u64>()
            .map_err(|_| de::Error::invalid_value(Unexpected::Str(value), &self))?;

        self.visit_u64(value)
    }
}

impl de::Visitor<'_> for VisitCommunityId {
    type Value = CommunityId;

    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "a community SteamID")
    }

    fn visit_u32<E: de::Error>(self, value: u32) -> Result<Self::Value, E> {
        CommunityId::new(value)
            .ok_or_else(|| de::Error::invalid_value(Unexpected::Unsigned(value.into()), &self))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if let Ok(value) = value.parse::<u32>() {
            return self.visit_u32(value);
        }

        SteamId::parse_community(value)
            .map(Into::into)
            .map_err(de::Error::custom)
    }
}

impl de::Visitor<'_> for VisitStandardFormat {
    type Value = SteamId;

    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "a SteamID")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        SteamId::parse_standard(value).map_err(de::Error::custom)
    }
}
