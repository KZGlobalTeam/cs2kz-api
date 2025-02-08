use std::fmt;

use crate::checksum::Checksum;
use crate::plugin::PluginVersionId;
use crate::{Context, database};

/// The official game modes supported by [`cs2kz-metamod`].
///
/// [`cs2kz-metamod`]: https://github.com/KZGlobalTeam/cs2kz-metamod
#[repr(u8)]
#[derive(Debug, Clone, Copy, serde::Serialize, sqlx::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Vanilla = 1,
    Classic = 2,
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn verify_checksum(
    cx: &Context,
    checksum: &Checksum,
    plugin_version_id: PluginVersionId,
) -> database::Result<bool> {
    sqlx::query_scalar!(
        "SELECT COUNT(mc.id) > 0 AS `is_valid: bool`
         FROM ModeChecksums AS mc
         JOIN PluginVersions AS v ON v.id = mc.plugin_version_id
         WHERE (mc.linux_checksum = ? OR mc.windows_checksum = ?)
         AND v.id = ?",
        checksum,
        checksum,
        plugin_version_id,
    )
    .fetch_one(cx.database().as_ref())
    .await
    .map_err(database::Error::from)
}

impl<'de> serde::Deserialize<'de> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Unexpected};

        struct ModeVisitor;

        impl de::Visitor<'_> for ModeVisitor {
            type Value = Mode;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a KZ mode")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    1 => Ok(Mode::Vanilla),
                    2 => Ok(Mode::Classic),
                    _ => Err(E::invalid_value(Unexpected::Unsigned(value), &self)),
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    "1" | "vnl" | "VNL" | "vanilla" | "Vanilla" => Ok(Mode::Vanilla),
                    "2" | "ckz" | "CKZ" | "classic" | "Classic" => Ok(Mode::Classic),
                    _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_any(ModeVisitor)
    }
}
