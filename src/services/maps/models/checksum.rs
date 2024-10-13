//! This module contains the [`Checksum`] types, representing a checksum of a
//! `.vpk` map file.

use std::slice::SliceIndex;
use std::{fmt, ops};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A map file's checksum.
///
/// Currently this uses the MD5 hashing algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, utoipa::ToSchema)]
#[schema(value_type = str)]
pub struct Checksum(md5::Digest);

impl Checksum
{
	/// Computes a new [`Checksum`] from the given bytes.
	pub fn new(bytes: &[u8]) -> Self
	{
		Self(md5::compute(bytes))
	}

	/// Returns the raw bytes of the checksum.
	pub fn as_bytes(&self) -> &[u8]
	{
		&self.0 .0[..]
	}

	/// Returns an element or range of the checksum's bytes, if the provided
	/// `index` is in-bounds.
	pub fn get<I>(&self, index: I) -> Option<&<Self as ops::Index<I>>::Output>
	where
		I: SliceIndex<[u8]>,
	{
		self.as_bytes().get(index)
	}
}

impl From<&[u8]> for Checksum
{
	fn from(bytes: &[u8]) -> Self
	{
		Self::new(bytes)
	}
}

impl From<md5::Digest> for Checksum
{
	fn from(digest: md5::Digest) -> Self
	{
		Self(digest)
	}
}

impl From<Checksum> for md5::Digest
{
	fn from(Checksum(digest): Checksum) -> Self
	{
		digest
	}
}

impl<I> ops::Index<I> for Checksum
where
	I: SliceIndex<[u8]>,
{
	type Output = <I as SliceIndex<[u8]>>::Output;

	fn index(&self, index: I) -> &Self::Output
	{
		self.as_bytes().index(index)
	}
}

impl fmt::Display for Checksum
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::LowerHex for Checksum
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::UpperHex for Checksum
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::UpperHex::fmt(&self.0, f)
	}
}

impl Serialize for Checksum
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		format_args!("{self}").serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Checksum
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(untagged)]
		enum Helper
		{
			Bytes([u8; 16]),

			#[serde(deserialize_with = "hex::serde::deserialize")]
			Hex([u8; 16]),
		}

		Helper::deserialize(deserializer).map(|v| match v {
			Helper::Bytes(bytes) | Helper::Hex(bytes) => Self(md5::Digest(bytes)),
		})
	}
}

impl<DB> sqlx::Type<DB> for Checksum
where
	DB: sqlx::Database,
	for<'a> &'a [u8]: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<&[u8] as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<&[u8] as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Checksum
where
	DB: sqlx::Database,
	for<'a> &'a [u8]: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
	{
		<&'_ [u8] as sqlx::Encode<'q, DB>>::encode_by_ref(&&self[..], buf)
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Checksum
where
	DB: sqlx::Database,
	&'r [u8]: sqlx::Decode<'r, DB>,
{
	fn decode(value: <DB as sqlx::Database>::ValueRef<'r>)
		-> Result<Self, sqlx::error::BoxDynError>
	{
		<&'r [u8] as sqlx::Decode<'r, DB>>::decode(value)?
			.try_into()
			.map(md5::Digest)
			.map(Self)
			.map_err(Into::into)
	}
}
