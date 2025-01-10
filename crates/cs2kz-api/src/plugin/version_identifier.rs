use std::fmt;

use cs2kz::git::GitRevision;
use utoipa::openapi::schema::{self, SchemaType};
use utoipa::openapi::{Object, OneOf, RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

#[derive(Debug)]
pub enum PluginVersionIdentifier {
    /// A SemVer version.
    SemVer(semver::Version),

    /// A git revision.
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

impl PartialSchema for PluginVersionIdentifier {
    fn schema() -> RefOr<Schema> {
        Schema::OneOf(
            OneOf::builder()
                .item(
                    Object::builder()
                        .title(Some("semver"))
                        .description(Some("a SemVer identifier"))
                        .schema_type(SchemaType::Type(schema::Type::String))
                        .examples(["1.23.456-dev"])
                        .build(),
                )
                .item(crate::openapi::shims::GitRevision::schema())
                .build(),
        )
        .into()
    }
}

impl ToSchema for PluginVersionIdentifier {}
