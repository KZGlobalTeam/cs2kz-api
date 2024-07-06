//! CS2 server hosts.
//!
//! CS2 servers are allowed to use both IPv4/IPv6 and full domain names. This module defines a
//! `Host` type that encapsulates any of these 3, and can encode/decode them properly.

use std::convert::Infallible;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::openapi::schema::KnownFormat;
use utoipa::openapi::{ObjectBuilder, OneOfBuilder, RefOr, Schema, SchemaFormat, SchemaType};
use utoipa::ToSchema;

/// A CS2 server host.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Host {
	/// An IPv4 address.
	Ipv4(Ipv4Addr),

	/// An IPv6 address.
	Ipv6(Ipv6Addr),

	/// A domain.
	Domain(String),
}

impl FromStr for Host {
	type Err = Infallible;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if let Ok(ip) = value.parse::<Ipv4Addr>() {
			return Ok(Self::Ipv4(ip));
		}

		if let Ok(ip) = value.parse::<Ipv6Addr>() {
			return Ok(Self::Ipv6(ip));
		}

		Ok(Self::Domain(value.to_owned()))
	}
}

impl<DB> sqlx::Type<DB> for Host
where
	DB: sqlx::Database,
	String: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		<String as sqlx::Type<DB>>::type_info()
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Host
where
	DB: sqlx::Database,
	String: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull {
		match self {
			Host::Ipv4(ip) => <String as sqlx::Encode<DB>>::encode_by_ref(&ip.to_string(), buf),
			Host::Ipv6(ip) => <String as sqlx::Encode<DB>>::encode_by_ref(&ip.to_string(), buf),
			Host::Domain(domain) => <String as sqlx::Encode<DB>>::encode_by_ref(domain, buf),
		}
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Host
where
	DB: sqlx::Database,
	&'r str: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError> {
		<&'r str as sqlx::Decode<'r, DB>>::decode(value)?
			.parse()
			.map_err(Into::into)
	}
}

impl<'s> ToSchema<'s> for Host {
	fn schema() -> (&'s str, RefOr<Schema>) {
		(
			"Host",
			Schema::OneOf(
				OneOfBuilder::new()
					.description(Some("A server's host address"))
					.nullable(false)
					.item(Schema::Object(
						ObjectBuilder::new()
							.title(Some("IPv4"))
							.schema_type(SchemaType::String)
							.example(Some("127.0.0.1".into()))
							.build(),
					))
					.item(Schema::Object(
						ObjectBuilder::new()
							.title(Some("IPv6"))
							.schema_type(SchemaType::String)
							.example(Some("::1".into()))
							.build(),
					))
					.item(Schema::Object(
						ObjectBuilder::new()
							.title(Some("Domain"))
							.schema_type(SchemaType::String)
							.format(Some(SchemaFormat::KnownFormat(KnownFormat::Uri)))
							.example(Some("example.com".into()))
							.build(),
					))
					.build(),
			)
			.into(),
		)
	}
}
