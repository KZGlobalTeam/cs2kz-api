//! Reasons for which players can get banned.

use std::time::Duration;
use std::{cmp, fmt};

use serde::{Deserialize, Serialize};

use crate::time::DurationExt;

/// Reasons for which players can get banned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BanReason
{
	/// Automated tick-perfect bhops.
	AutoBhop,

	/// Automated perfect airstrafes.
	AutoStrafe,

	/// Some kind of macro to automate parts of movement.
	Macro,
}

impl BanReason
{
	/// Returns a string representation of this ban reason.
	pub fn as_str(&self) -> &'static str
	{
		match self {
			BanReason::AutoBhop => "auto_bhop",
			BanReason::AutoStrafe => "auto_strafe",
			BanReason::Macro => "macro",
		}
	}

	/// Determines the duration for a ban given the ban reason.
	///
	/// `previous_ban_duration` represents the sum of the durations of all
	/// previous non-false bans.
	pub fn duration(&self, previous_ban_duration: Option<Duration>) -> Duration
	{
		let base_duration = match self {
			BanReason::AutoBhop => Duration::MONTH * 2,
			BanReason::AutoStrafe => Duration::MONTH,
			BanReason::Macro => Duration::WEEK * 2,
		};

		let final_duration =
			previous_ban_duration.map_or(base_duration, |duration| (base_duration + duration) * 2);

		cmp::min(Duration::YEAR, final_duration)
	}
}

impl fmt::Display for BanReason
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.write_str(self.as_str())
	}
}

impl<DB> sqlx::Type<DB> for BanReason
where
	DB: sqlx::Database,
	str: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<str as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<str as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for BanReason
where
	DB: sqlx::Database,
	for<'a> &'a str: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
	{
		<&'_ str as sqlx::Encode<'q, DB>>::encode_by_ref(&self.as_str(), buf)
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for BanReason
where
	DB: sqlx::Database,
	&'r str: sqlx::Decode<'r, DB>,
{
	fn decode(value: <DB as sqlx::Database>::ValueRef<'r>)
		-> Result<Self, sqlx::error::BoxDynError>
	{
		match <&'r str as sqlx::Decode<'r, DB>>::decode(value)? {
			"auto_bhop" => Ok(Self::AutoBhop),
			"auto_strafe" => Ok(Self::AutoStrafe),
			"macro" => Ok(Self::Macro),
			_ => Err("invalid ban reason".into()),
		}
	}
}
