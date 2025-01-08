use std::fmt;
use std::num::NonZero;

use cs2kz::maps::MapId;
use serde::de::{self, Deserialize, Deserializer, Unexpected};

#[derive(Debug, utoipa::ToSchema)]
pub enum MapIdentifier {
    #[schema(value_type = u16)]
    Id(MapId),
    Name(String),
}

impl<'de> Deserialize<'de> for MapIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MapIdentifierVisitor;

        impl de::Visitor<'_> for MapIdentifierVisitor {
            type Value = MapIdentifier;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a map ID or name")
            }

            fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                NonZero::new(value)
                    .map(MapId::from_inner)
                    .map(MapIdentifier::Id)
                    .ok_or_else(|| {
                        E::invalid_value(Unexpected::Unsigned(value.into()), &"a non-zero map ID")
                    })
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(map_id) = value.parse::<MapId>() {
                    return Ok(MapIdentifier::Id(map_id));
                }

                Ok(MapIdentifier::Name(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(map_id) = value.parse::<MapId>() {
                    return Ok(MapIdentifier::Id(map_id));
                }

                Ok(MapIdentifier::Name(value))
            }
        }

        deserializer.deserialize_any(MapIdentifierVisitor)
    }
}
