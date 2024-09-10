//! This module contains extensions for [`std::net`].

#![expect(clippy::disallowed_types, reason = "this module implements the replacement wrappers")]

use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper around [`std::net::Ipv6Addr`] that correctly takes care of mapped
/// IPv4 addresses when encoding/decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, utoipa::ToSchema)]
#[schema(value_type = str)]
pub struct IpAddr(Ipv6Addr);

impl fmt::Display for IpAddr
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Display::fmt(&self.0, f)
	}
}

impl From<std::net::IpAddr> for IpAddr
{
	fn from(ip: std::net::IpAddr) -> Self
	{
		match ip {
			std::net::IpAddr::V4(ipv4) => ipv4.into(),
			std::net::IpAddr::V6(ipv6) => ipv6.into(),
		}
	}
}

impl From<Ipv4Addr> for IpAddr
{
	fn from(ipv4: Ipv4Addr) -> Self
	{
		Self(ipv4.to_ipv6_mapped())
	}
}

impl From<Ipv6Addr> for IpAddr
{
	fn from(ipv6: Ipv6Addr) -> Self
	{
		Self(ipv6)
	}
}

impl FromStr for IpAddr
{
	type Err = <std::net::IpAddr as FromStr>::Err;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<std::net::IpAddr>().map(Into::into)
	}
}

impl Serialize for IpAddr
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.0.to_canonical().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for IpAddr
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		std::net::IpAddr::deserialize(deserializer).map(Into::into)
	}
}

impl<DB> sqlx::Type<DB> for IpAddr
where
	DB: sqlx::Database,
	Ipv6Addr: sqlx::Type<DB>,
	for<'a> &'a [u8]: sqlx::Type<DB>,
{
	fn type_info() -> DB::TypeInfo
	{
		<Ipv6Addr as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &DB::TypeInfo) -> bool
	{
		<Ipv6Addr as sqlx::Type<DB>>::compatible(ty) || <&[u8] as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for IpAddr
where
	DB: sqlx::Database,
	Ipv6Addr: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
	{
		<Ipv6Addr as sqlx::Encode<'q, DB>>::encode_by_ref(&self.0, buf)
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for IpAddr
where
	DB: sqlx::Database,
	Ipv6Addr: sqlx::Decode<'r, DB>,
{
	fn decode(value: <DB as sqlx::Database>::ValueRef<'r>)
	-> Result<Self, sqlx::error::BoxDynError>
	{
		<Ipv6Addr as sqlx::Decode<'r, DB>>::decode(value).map(Self)
	}
}
