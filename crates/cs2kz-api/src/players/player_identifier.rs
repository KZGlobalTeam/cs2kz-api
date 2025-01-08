use std::fmt;

use cs2kz::players::PlayerId;
use serde::de::{self, Deserialize, Deserializer};
use steam_id::SteamId;

#[derive(Debug, utoipa::ToSchema)]
pub enum PlayerIdentifier {
    /// A SteamID.
    #[schema(value_type = crate::openapi::shims::SteamId)]
    Id(PlayerId),

    /// A name.
    Name(String),
}

impl<'de> Deserialize<'de> for PlayerIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PlayerIdentifierVisitor;

        impl de::Visitor<'_> for PlayerIdentifierVisitor {
            type Value = PlayerIdentifier;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a player ID or name")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                SteamId::from_u64(value)
                    .map(PlayerId::new)
                    .map(PlayerIdentifier::Id)
                    .map_err(E::custom)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(steam_id) = value.parse::<SteamId>() {
                    return Ok(PlayerIdentifier::Id(PlayerId::new(steam_id)));
                }

                Ok(PlayerIdentifier::Name(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(steam_id) = value.parse::<SteamId>() {
                    return Ok(PlayerIdentifier::Id(PlayerId::new(steam_id)));
                }

                Ok(PlayerIdentifier::Name(value))
            }
        }

        deserializer.deserialize_any(PlayerIdentifierVisitor)
    }
}
