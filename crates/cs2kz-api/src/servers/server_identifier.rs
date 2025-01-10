use std::fmt;
use std::num::NonZero;

use cs2kz::servers::ServerId;
use serde::de::{self, Deserialize, Deserializer, Unexpected};
use utoipa::openapi::schema::{self, KnownFormat, SchemaFormat, SchemaType};
use utoipa::openapi::{Object, OneOf, RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

#[derive(Debug)]
pub enum ServerIdentifier {
    Id(ServerId),
    Name(String),
}

impl<'de> Deserialize<'de> for ServerIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ServerIdentifierVisitor;

        impl de::Visitor<'_> for ServerIdentifierVisitor {
            type Value = ServerIdentifier;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a server ID or name")
            }

            fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                NonZero::new(value)
                    .map(ServerId::from_inner)
                    .map(ServerIdentifier::Id)
                    .ok_or_else(|| {
                        E::invalid_value(
                            Unexpected::Unsigned(value.into()),
                            &"a non-zero server ID",
                        )
                    })
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(server_id) = value.parse::<ServerId>() {
                    return Ok(ServerIdentifier::Id(server_id));
                }

                Ok(ServerIdentifier::Name(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(server_id) = value.parse::<ServerId>() {
                    return Ok(ServerIdentifier::Id(server_id));
                }

                Ok(ServerIdentifier::Name(value))
            }
        }

        deserializer.deserialize_any(ServerIdentifierVisitor)
    }
}

impl PartialSchema for ServerIdentifier {
    fn schema() -> RefOr<Schema> {
        Schema::OneOf(
            OneOf::builder()
                .item(
                    Object::builder()
                        .title(Some("name"))
                        .schema_type(SchemaType::Type(schema::Type::String))
                        .examples(["Alpha's KZ"])
                        .build(),
                )
                .item(
                    Object::builder()
                        .title(Some("id"))
                        .schema_type(SchemaType::Type(schema::Type::Integer))
                        .format(Some(SchemaFormat::KnownFormat(KnownFormat::UInt16)))
                        .examples(["69"]),
                )
                .build(),
        )
        .into()
    }
}

impl ToSchema for ServerIdentifier {}
