use std::fmt;
use std::future::Future;
use std::result::Result as StdResult;

use cs2kz::{PlayerIdentifier, ServerIdentifier, SteamID};
use serde::{Deserialize, Serialize};
use sqlx::MySqlExecutor;
use thiserror::Error as ThisError;
use utoipa::ToSchema;

use crate::{Error, Result};

/// Utility trait for turning enums that might be an "ID" into an ID.
pub trait ToID {
	/// The ID that this type can be turned into.
	type ID;

	/// The function that turns this type into an ID.
	fn to_id<'c>(
		&self,
		executor: impl MySqlExecutor<'c>,
	) -> impl Future<Output = Result<Self::ID>> + Send;
}

impl ToID for PlayerIdentifier<'_> {
	type ID = SteamID;

	async fn to_id<'c>(&self, executor: impl MySqlExecutor<'c>) -> Result<SteamID> {
		match *self {
			PlayerIdentifier::SteamID(steam_id) => Ok(steam_id),
			PlayerIdentifier::Name(ref name) => sqlx::query! {
				r#"
				SELECT
				  steam_id `steam_id: SteamID`
				FROM
				  Players
				WHERE
				  name LIKE ?
				"#,
				format!("%{name}%"),
			}
			.fetch_optional(executor)
			.await?
			.map(|row| row.steam_id)
			.ok_or(Error::NoContent),
		}
	}
}

impl ToID for ServerIdentifier<'_> {
	type ID = u16;

	async fn to_id<'c>(&self, executor: impl MySqlExecutor<'c>) -> Result<u16> {
		match *self {
			ServerIdentifier::ID(id) => Ok(id),
			ServerIdentifier::Name(ref name) => sqlx::query! {
				r#"
				SELECT
				  id
				FROM
				  Servers
				WHERE
				  name LIKE ?
				"#,
				format!("%{name}%"),
			}
			.fetch_optional(executor)
			.await?
			.map(|row| row.id)
			.ok_or(Error::NoContent),
		}
	}
}

/// Ranked status of a [`Filter`].
///
/// [`Filter`]: crate::maps::models::Filter
#[repr(i8)]
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum RankedStatus {
	/// The filter will never be ranked, either because the mapper requested so, or because
	/// it is not deemed worthy for ranking.
	Never = -1,

	/// The filter is not currently ranked, but might be in the future.
	Unranked = 0,

	/// The filter is currently ranked.
	Ranked = 1,
}

#[derive(Debug, ThisError)]
#[error("`{0}` is not a valid ranked status.")]
pub struct InvalidRankedStatus(i8);

impl TryFrom<i8> for RankedStatus {
	type Error = InvalidRankedStatus;

	fn try_from(value: i8) -> StdResult<Self, Self::Error> {
		match value {
			-1 => Ok(Self::Never),
			0 => Ok(Self::Unranked),
			1 => Ok(Self::Ranked),
			invalid => Err(InvalidRankedStatus(invalid)),
		}
	}
}

/// Global status of a map.
#[repr(i8)]
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GlobalStatus {
	/// The map is currently not global.
	NotGlobal = -1,

	/// The map is currently in a testing phase.
	///
	/// This means it is not finished yet, and records set on it will be wiped once the map
	/// gets ranked.
	InTesting = 0,

	/// The map is currently global.
	Global = 1,
}

impl fmt::Display for GlobalStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			GlobalStatus::NotGlobal => "not global",
			GlobalStatus::InTesting => "in testing",
			GlobalStatus::Global => "global",
		})
	}
}
