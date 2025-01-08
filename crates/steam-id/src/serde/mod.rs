use serde::de::{self, Deserialize, Deserializer, Unexpected};
use serde::ser::{Serialize, Serializer};

use crate::{CommunityId, SteamId};

mod visitors;
pub use visitors::{VisitCommunityId, VisitEverything, VisitStandardFormat, VisitU64};

impl SteamId {
    pub fn serialize_u64<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_u64().serialize(serializer)
    }

    pub fn serialize_u64_stringified<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format_args!("{}", self.as_u64()).serialize(serializer)
    }

    pub fn serialize_standard<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format_args!("{self}").serialize(serializer)
    }

    pub fn serialize_as_community_id<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_community_id().serialize(serializer)
    }

    pub fn serialize_as_community_id_with_brackets<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_community_id().serialize_with_brackets(serializer)
    }

    pub fn serialize_as_community_id_without_brackets<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_community_id()
            .serialize_without_brackets(serializer)
    }
}

impl Serialize for SteamId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.serialize_standard(serializer)
    }
}

impl SteamId {
    pub fn deserialize_u64<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_u64(VisitU64::default())
    }

    pub fn deserialize_standard<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(VisitStandardFormat::default())
    }

    pub fn deserialize_community<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_str(VisitCommunityId::default())
            .map(Into::into)
    }
}

impl<'de> Deserialize<'de> for SteamId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(VisitEverything::default())
    }
}

impl CommunityId {
    pub fn serialize_with_brackets<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format_args!("[U:1:{self}]").serialize(serializer)
    }

    pub fn serialize_without_brackets<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format_args!("U:1:{self}").serialize(serializer)
    }
}

impl Serialize for CommunityId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.get().serialize(serializer)
    }
}

impl CommunityId {
    pub fn deserialize_u32<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;

        CommunityId::new(value).ok_or_else(|| {
            de::Error::invalid_value(Unexpected::Unsigned(value.into()), &"a 32-bit SteamID")
        })
    }
}

impl<'de> Deserialize<'de> for CommunityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(VisitCommunityId::default())
    }
}
