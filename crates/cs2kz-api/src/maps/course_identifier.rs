use std::fmt;
use std::num::NonZero;

use serde::de::{self, Deserialize, Deserializer, Unexpected};
use utoipa::openapi::schema::{self, KnownFormat, SchemaFormat, SchemaType};
use utoipa::openapi::{Object, OneOf, RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

#[derive(Debug)]
pub enum CourseIdentifier {
    Number(NonZero<u16>),
    Name(String),
}

impl<'de> Deserialize<'de> for CourseIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CourseIdentifierVisitor;

        impl de::Visitor<'_> for CourseIdentifierVisitor {
            type Value = CourseIdentifier;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a course ID or name")
            }

            fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                NonZero::new(value)
                    .map(CourseIdentifier::Number)
                    .ok_or_else(|| {
                        E::invalid_value(
                            Unexpected::Unsigned(value.into()),
                            &"a non-zero course ID",
                        )
                    })
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(course_number) = value.parse::<u16>() {
                    return self.visit_u16(course_number);
                }

                Ok(CourseIdentifier::Name(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(course_number) = value.parse::<u16>() {
                    return self.visit_u16(course_number);
                }

                Ok(CourseIdentifier::Name(value))
            }
        }

        deserializer.deserialize_any(CourseIdentifierVisitor)
    }
}

impl PartialSchema for CourseIdentifier {
    fn schema() -> RefOr<Schema> {
        Schema::OneOf(
            OneOf::builder()
                .item(
                    Object::builder()
                        .title(Some("name"))
                        .schema_type(SchemaType::Type(schema::Type::String))
                        .examples(["kz_checkmate"])
                        .build(),
                )
                .item(
                    Object::builder()
                        .title(Some("number"))
                        .schema_type(SchemaType::Type(schema::Type::Integer))
                        .format(Some(SchemaFormat::KnownFormat(KnownFormat::UInt16)))
                        .examples(["69"]),
                )
                .build(),
        )
        .into()
    }
}

impl ToSchema for CourseIdentifier {}
