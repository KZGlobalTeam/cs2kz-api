use std::fmt;

#[repr(u8)]
#[derive(Debug, Clone, Copy, sqlx::Type, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum JumpType {
    LongJump,
    Bhop,
    MultiBhop,
    WeirdJump,
    LadderJump,
    Ladderhop,
    Jumpbug,
    Fall,
}

impl<'de> serde::Deserialize<'de> for JumpType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Unexpected};

        struct JumpTypeVisitor;

        impl de::Visitor<'_> for JumpTypeVisitor {
            type Value = JumpType;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a jump type")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    0 => Ok(JumpType::LongJump),
                    1 => Ok(JumpType::Bhop),
                    2 => Ok(JumpType::MultiBhop),
                    3 => Ok(JumpType::WeirdJump),
                    4 => Ok(JumpType::LadderJump),
                    5 => Ok(JumpType::Ladderhop),
                    6 => Ok(JumpType::Jumpbug),
                    7 => Ok(JumpType::Fall),
                    _ => Err(E::invalid_value(Unexpected::Unsigned(value), &self)),
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(int) = value.parse::<u64>() {
                    return self.visit_u64(int);
                }

                match value {
                    "long-jump" => Ok(JumpType::LongJump),
                    "bhop" => Ok(JumpType::Bhop),
                    "multi-bhop" => Ok(JumpType::MultiBhop),
                    "weird-jump" => Ok(JumpType::WeirdJump),
                    "ladder-jump" => Ok(JumpType::LadderJump),
                    "ladderhop" => Ok(JumpType::Ladderhop),
                    "jumpbug" => Ok(JumpType::Jumpbug),
                    "fall" => Ok(JumpType::Fall),
                    _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_any(JumpTypeVisitor)
    }
}
