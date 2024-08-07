//! A service for managing player bans.

/* TODO:
 * - allow attaching notes to bans
 *    - allow including notes when submitting bans
 *    - allow updating notes when updating bans
 */

use std::fmt;
use std::time::Duration;

use axum::extract::FromRef;
use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use sqlx::{MySql, Pool, Transaction};
use tap::Conv;

use crate::database::{SqlErrorExt, TransactionExt};
use crate::net::IpAddr;
use crate::services::plugin::PluginVersionID;
use crate::services::servers::ServerID;
use crate::services::AuthService;

pub(crate) mod http;
mod queries;

mod error;
pub use error::{Error, Result};

pub(crate) mod models;
pub use models::{
	BanID,
	BanReason,
	BanRequest,
	BanResponse,
	BannedBy,
	FetchBanRequest,
	FetchBanResponse,
	FetchBansRequest,
	FetchBansResponse,
	Unban,
	UnbanID,
	UnbanReason,
	UnbanRequest,
	UnbanResponse,
	UpdateBanRequest,
	UpdateBanResponse,
};

/// A service for managing player bans.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct BanService
{
	database: Pool<MySql>,
	auth_svc: AuthService,
}

impl fmt::Debug for BanService
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("BanService").finish_non_exhaustive()
	}
}

impl BanService
{
	/// Create a new [`BanService`].
	#[tracing::instrument]
	pub fn new(database: Pool<MySql>, auth_svc: AuthService) -> Self
	{
		Self { database, auth_svc }
	}

	/// Fetch a ban.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_ban(&self, req: FetchBanRequest) -> Result<Option<FetchBanResponse>>
	{
		let res = sqlx::query_as::<_, FetchBanResponse>(&format!(
			r"
			{}
			WHERE
			  b.id = ?
			LIMIT
			  1
			",
			queries::SELECT,
		))
		.bind(req.ban_id)
		.fetch_optional(&self.database)
		.await?;

		Ok(res)
	}

	/// Fetch many bans.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_bans(&self, req: FetchBansRequest) -> Result<FetchBansResponse>
	{
		let mut txn = self.database.begin().await?;

		let player_id = match req.player {
			None => None,
			Some(ident) => ident.resolve_id(txn.as_mut()).await?,
		};

		let server_id = match req.server {
			None => None,
			Some(ident) => ident.resolve_id(txn.as_mut()).await?,
		};

		let bans = sqlx::query_as::<_, FetchBanResponse>(&format!(
			r"
			{}
			WHERE
			  b.player_id = COALESCE(?, b.player_id)
			  AND (
			    (? IS NULL)
			    OR b.server_id = ?
			  )
			  AND b.reason = COALESCE(?, b.reason)
			  AND b.created_on > COALESCE(?, '1970-01-01 00:00:01')
			  AND b.created_on < COALESCE(?, '2038-01-19 03:14:07')
			  AND ({})
			LIMIT
			  ? OFFSET ?
			",
			queries::SELECT,
			match req.unbanned {
				None => "true",
				Some(false) => "ub.id IS NULL",
				Some(true) => "ub.id IS NOT NULL",
			},
		))
		.bind(player_id)
		.bind(server_id)
		.bind(server_id)
		.bind(req.reason)
		.bind(req.created_after)
		.bind(req.created_before)
		.bind(*req.limit)
		.bind(*req.offset)
		.fetch_all(txn.as_mut())
		.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchBansResponse { bans, total })
	}

	/// Ban a player.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn ban_player(&self, req: BanRequest) -> Result<BanResponse>
	{
		let mut txn = self.database.begin().await?;

		let ban_duration = calculate_ban_duration(req.player_id, req.reason, &mut txn).await?;
		let player_ip = resolve_player_ip(req.player_ip, req.player_id, &mut txn).await?;
		let banned_by_details = banned_by_details(req.banned_by, &mut txn).await?;

		let ban_id = create_ban(
			req.player_id,
			player_ip,
			req.reason,
			&banned_by_details,
			ban_duration,
			&mut txn,
		)
		.await?;

		txn.commit().await?;

		tracing::trace! {
			target: "cs2kz_api::audit_log",
			%ban_id,
			player_id = %req.player_id,
			reason = %req.reason,
			server_id = ?banned_by_details.server_id,
			admin_id = ?banned_by_details.admin_id,
			?ban_duration,
			"issued ban",
		};

		Ok(BanResponse { ban_id })
	}

	/// Update a ban.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn update_ban(&self, req: UpdateBanRequest) -> Result<UpdateBanResponse>
	{
		if req.is_empty() {
			return Ok(UpdateBanResponse { _priv: () });
		}

		let mut txn = self.database.begin().await?;

		let (created_on, unban_id) = sqlx::query! {
			r"
			SELECT
			  b.created_on `created_on: DateTime<Utc>`,
			  ub.id `unban_id: UnbanID`
			FROM
			  Bans b
			  LEFT JOIN Unbans ub ON ub.ban_id = b.id
			WHERE
			  b.id = ?
			",
			req.ban_id,
		}
		.fetch_optional(txn.as_mut())
		.await?
		.map(|row| (row.created_on, row.unban_id))
		.ok_or(Error::BanDoesNotExist { ban_id: req.ban_id })?;

		if matches!(req.new_expiration_date, Some(date) if date < created_on) {
			return Err(Error::ExpirationBeforeCreation);
		}

		if let Some(unban_id) = unban_id {
			return Err(Error::BanAlreadyReverted { unban_id });
		}

		let query_result = sqlx::query! {
			r"
			UPDATE
			  Bans
			SET
			  reason = COALESCE(?, reason),
			  expires_on = COALESCE(?, expires_on)
			WHERE
			  id = ?
			",
			req.new_reason,
			req.new_expiration_date,
			req.ban_id,
		}
		.execute(txn.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::BanDoesNotExist { ban_id: req.ban_id }),
			n => assert_eq!(n, 1, "updated more than 1 ban"),
		}

		txn.commit().await?;

		tracing::debug!(target: "cs2kz_api::audit_log", ban_id = %req.ban_id, "updated ban");

		Ok(UpdateBanResponse { _priv: () })
	}

	/// Unban a player.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn unban_player(&self, req: UnbanRequest) -> Result<UnbanResponse>
	{
		let existing_unban = sqlx::query_scalar! {
			r"
			SELECT
			  id `id: UnbanID`
			FROM
			  Unbans
			WHERE
			  ban_id = ?
			",
			req.ban_id,
		}
		.fetch_optional(&self.database)
		.await?;

		if let Some(unban_id) = existing_unban {
			return Err(Error::BanAlreadyReverted { unban_id });
		}

		let mut txn = self.database.begin().await?;

		let query_result = sqlx::query! {
			r"
			UPDATE
			  Bans
			SET
			  expires_on = NOW()
			WHERE
			  id = ?
			",
			req.ban_id,
		}
		.execute(txn.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::BanDoesNotExist { ban_id: req.ban_id }),
			n => assert_eq!(n, 1, "updated more than 1 ban"),
		}

		let unban_id = sqlx::query! {
			r"
			INSERT INTO
			  Unbans (ban_id, reason, admin_id)
			VALUES
			  (?, ?, ?)
			",
			req.ban_id,
			req.reason,
			req.admin_id,
		}
		.execute(txn.as_mut())
		.await?
		.last_insert_id()
		.conv::<UnbanID>();

		txn.commit().await?;

		tracing::debug! {
			target: "cs2kz_api::audit_log",
			ban_id = %req.ban_id,
			%unban_id,
			admin_id = %req.admin_id,
			"reverted ban",
		};

		Ok(UnbanResponse { ban_id: req.ban_id, unban_id })
	}
}

/// Calculates the ban duration for a new ban for a given player for a given
/// reason.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
async fn calculate_ban_duration(
	player_id: SteamID,
	reason: BanReason,
	txn: &mut Transaction<'_, MySql>,
) -> Result<Duration>
{
	let (currently_banned, has_previous_bans, previous_ban_duration) = sqlx::query! {
		r"
		SELECT
		  COUNT(active_bans.id) > 0 `currently_banned: bool`,
		  COUNT(expired_bans.id) > 0 `has_previous_bans: bool`,
		  TIMESTAMPDIFF(
		    SECOND,
		    expired_bans.created_on,
		    expired_bans.expires_on
		  ) `previous_ban_duration: u64`
		FROM
		  Players p
		  LEFT JOIN Bans active_bans ON active_bans.player_id = p.id
		  AND active_bans.expires_on > NOW()
		  LEFT JOIN Bans expired_bans ON expired_bans.player_id = p.id
		  AND expired_bans.expires_on < NOW()
		  AND expired_bans.id IN (
		    SELECT
		      ban_id
		    FROM
		      Unbans
		    WHERE
		      reason != 'false_ban'
		  )
		WHERE
		  p.id = ?
		",
		player_id,
	}
	.fetch_optional(txn.as_mut())
	.await?
	.map(|row| (row.currently_banned, row.has_previous_bans, row.previous_ban_duration))
	.unwrap_or_default();

	match (currently_banned, has_previous_bans, previous_ban_duration) {
		// This is the player's first ever ban
		(false, false, previous_ban_duration @ None)
		// The player isn't currently banned but was banned in the past
		| (false, true, previous_ban_duration @ Some(_)) => {
			Ok(reason.duration(previous_ban_duration.map(Duration::from_secs)))
		}

		// The player isn't currently banned, has never been banned, but has a
		// total ban duration...?
		(false, false, Some(_)) => {
			unreachable!("cannot have ban duration without bans");
		}

		// The player is currently banned, was never banned in the past, but has
		// a previous ban duration?
		(true, false, Some(_)) => {
			unreachable!("cannot be currently banned with 0 previous bans");
		}

		// The player is not currently banned, was banned in the past, but has no
		// total ban duration...?
		(false, true, None) => {
			unreachable!("cannot be not-banned and have perma ban at the same time");
		}

		// Player is currently banned, so we can't ban them again
		(true, ..) => Err(Error::PlayerAlreadyBanned { steam_id: player_id }),
	}
}

/// Resolves a player's IP address by mapping IPv4 to IPv6 or fetching the
/// missing IP from the database.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
async fn resolve_player_ip(
	player_ip: Option<IpAddr>,
	player_id: SteamID,
	txn: &mut Transaction<'_, MySql>,
) -> Result<IpAddr>
{
	Ok(match player_ip {
		Some(ip) => ip,
		None => sqlx::query_scalar! {
			r"
			SELECT
			  ip_address `ip: IpAddr`
			FROM
			  Players
			WHERE
			  id = ?
			LIMIT
			  1
			",
			player_id,
		}
		.fetch_optional(txn.as_mut())
		.await?
		.ok_or(Error::PlayerDoesNotExist { steam_id: player_id })?,
	})
}

#[derive(Debug)]
#[allow(clippy::missing_docs_in_private_items)]
struct BannedByDetails
{
	server_id: Option<ServerID>,
	admin_id: Option<SteamID>,
	plugin_version_id: PluginVersionID,
}

/// Extracts the relevant details out of a [`BannedBy`] and fetches additional
/// information from the database.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
async fn banned_by_details(
	banned_by: BannedBy,
	txn: &mut Transaction<'_, MySql>,
) -> Result<BannedByDetails>
{
	Ok(match banned_by {
		BannedBy::Server { id, plugin_version_id } => {
			BannedByDetails { server_id: Some(id), admin_id: None, plugin_version_id }
		}
		BannedBy::Admin { steam_id } => BannedByDetails {
			server_id: None,
			admin_id: Some(steam_id),
			plugin_version_id: sqlx::query_scalar! {
				r"
				SELECT
				  id `id: PluginVersionID`
				FROM
				  PluginVersions
				ORDER BY
				  created_on DESC
				LIMIT
				  1
				",
			}
			.fetch_one(txn.as_mut())
			.await?,
		},
	})
}

/// Creates a new ban in the database and returns its ID.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
async fn create_ban(
	player_id: SteamID,
	player_ip: IpAddr,
	reason: BanReason,
	banned_by_details: &BannedByDetails,
	ban_duration: Duration,
	txn: &mut Transaction<'_, MySql>,
) -> Result<BanID>
{
	Ok(sqlx::query! {
		r"
		INSERT INTO
		  Bans(
		    player_id,
		    player_ip,
		    server_id,
		    reason,
		    admin_id,
		    plugin_version_id,
		    expires_on
		  )
		VALUES
		(?, ?, ?, ?, ?, ?, ?)
		",
		player_id,
		player_ip,
		banned_by_details.server_id,
		reason,
		banned_by_details.admin_id,
		banned_by_details.plugin_version_id,
		Utc::now() + ban_duration,
	}
	.execute(txn.as_mut())
	.await
	.map_err(|error| {
		if error.is_fk_violation("player_id") {
			Error::PlayerDoesNotExist { steam_id: player_id }
		} else if error.is_fk_violation("admin_id") {
			Error::PlayerDoesNotExist {
				steam_id: banned_by_details
					.admin_id
					.expect("we need a non-null admin_id to get this conflict"),
			}
		} else {
			Error::Database(error)
		}
	})?
	.last_insert_id()
	.conv::<BanID>())
}

#[cfg(test)]
mod tests
{
	use sqlx::{MySql, Pool};

	use super::*;
	use crate::testing;

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/bans.sql")
	)]
	async fn fetch_ban_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::ban_svc(database);
		let req = FetchBanRequest { ban_id: 1.into() };
		let res = svc.fetch_ban(req).await?.expect("there should be a ban");

		testing::assert_eq!(res.player.name, "iBrahizy");
		testing::assert_eq!(res.admin.as_ref().map(|p| &*p.name), Some("AlphaKeks"));
		testing::assert_eq!(res.reason, BanReason::AutoBhop);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn fetch_ban_not_found(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::ban_svc(database);
		let req = FetchBanRequest { ban_id: 1.into() };
		let res = svc.fetch_ban(req).await?;

		testing::assert!(res.is_none());

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/bans.sql")
	)]
	async fn fetch_bans_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::ban_svc(database);
		let req = FetchBansRequest::default();
		let res = svc.fetch_bans(req).await?;

		testing::assert_eq!(res.bans.len(), 3);
		testing::assert_eq!(res.total, 3);

		let req = FetchBansRequest { player: Some("alphakeks".parse()?), ..Default::default() };
		let res = svc.fetch_bans(req).await?;

		testing::assert_eq!(res.bans.len(), 1);
		testing::assert_eq!(res.total, 1);
		testing::assert!(res.bans[0].server.is_some());
		testing::assert!(res.bans[0].admin.is_none());

		let req = FetchBansRequest { reason: Some(BanReason::AutoStrafe), ..Default::default() };
		let res = svc.fetch_bans(req).await?;

		testing::assert_eq!(res.bans.len(), 1);
		testing::assert_eq!(res.total, 1);
		testing::assert_eq!(res.bans[0].player.name, "zer0.k");

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn fetch_bans_no_content(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::ban_svc(database);
		let req = FetchBansRequest::default();
		let res = svc.fetch_bans(req).await?;

		testing::assert_eq!(res.bans.len(), 0);
		testing::assert_eq!(res.total, 0);

		Ok(())
	}
}
