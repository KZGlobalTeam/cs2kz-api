use std::fmt;

use cs2kz::git::GitRevision;

#[derive(Debug, utoipa::ToSchema)]
pub enum PluginVersionIdentifier {
    /// A SemVer version.
    #[schema(value_type = str)]
    SemVer(semver::Version),

    /// A git revision.
    #[schema(value_type = crate::openapi::shims::GitRevision)]
    GitRevision(GitRevision),
}

impl<'de> serde::Deserialize<'de> for PluginVersionIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Unexpected};

        struct PluginVersionIdentifierVisitor;

        impl de::Visitor<'_> for PluginVersionIdentifierVisitor {
            type Value = PluginVersionIdentifier;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a SemVer version or git revision")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(version) = value.parse::<semver::Version>() {
                    return Ok(PluginVersionIdentifier::SemVer(version));
                }

                if let Ok(git_revision) = value.parse::<GitRevision>() {
                    return Ok(PluginVersionIdentifier::GitRevision(git_revision));
                }

                Err(E::invalid_value(Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(PluginVersionIdentifierVisitor)
    }
}
