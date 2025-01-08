use std::fmt;

/// The different states a map can be in.
#[repr(i8)]
#[derive(Debug, Clone, Copy, sqlx::Type, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MapState {
    /// The map has either been disabled manually, or it is an older version of a map.
    Invalid = -1,

    /// The map is currently in the public testing phase.
    InTesting = 0,

    /// The map has been approved and players can submit records on its courses.
    Approved = 1,
}

impl<'de> serde::Deserialize<'de> for MapState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Unexpected};

        struct MapStateVisitor;

        impl de::Visitor<'_> for MapStateVisitor {
            type Value = MapState;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a map state")
            }

            fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    -1 => Ok(MapState::Invalid),
                    0 => Ok(MapState::InTesting),
                    1 => Ok(MapState::Approved),
                    _ => Err(E::invalid_value(Unexpected::Signed(value.into()), &self)),
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(int) = value.parse::<i8>() {
                    return self.visit_i8(int);
                }

                match value {
                    "invalid" => Ok(MapState::Invalid),
                    "in-testing" => Ok(MapState::InTesting),
                    "approved" => Ok(MapState::Approved),
                    _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_any(MapStateVisitor)
    }
}
