//! A service for managing jumpstats.

/* TODO:
 * - run new submissions through anti-cheat service
 * - include replays in submitted jumpstats
 *    - allow downloading the replay for a jumpstat
 */

use std::fmt;

use axum::extract::FromRef;
use sqlx::{MySql, Pool};
use tap::Conv;

pub(crate) mod http;
mod queries;

mod error;
pub use error::{Error, Result};

pub(crate) mod models;
pub use models::{
	FetchJumpstatRequest,
	FetchJumpstatResponse,
	FetchJumpstatsRequest,
	FetchJumpstatsResponse,
	JumpstatID,
	SubmitJumpstatRequest,
	SubmitJumpstatResponse,
};

use crate::database::{SqlErrorExt, TransactionExt};
use crate::services::AuthService;

/// A service for managing jumpstats.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct JumpstatService
{
	database: Pool<MySql>,
	auth_svc: AuthService,
}

impl fmt::Debug for JumpstatService
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("JumpstatService").finish_non_exhaustive()
	}
}

impl JumpstatService
{
	/// Create a new [`JumpstatService`].
	#[tracing::instrument]
	pub fn new(database: Pool<MySql>, auth_svc: AuthService) -> Self
	{
		Self { database, auth_svc }
	}

	/// Fetch a jumpstat.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_jumpstat(
		&self,
		req: FetchJumpstatRequest,
	) -> Result<Option<FetchJumpstatResponse>>
	{
		let jumpstat = sqlx::query_as::<_, FetchJumpstatResponse>(&format!(
			r"
			{}
			WHERE
			  j.id = ?
			LIMIT
			  1
			",
			queries::SELECT,
		))
		.bind(req.jumpstat_id)
		.fetch_optional(&self.database)
		.await?;

		Ok(jumpstat)
	}

	/// Fetch jumpstats.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_jumpstats(
		&self,
		req: FetchJumpstatsRequest,
	) -> Result<FetchJumpstatsResponse>
	{
		let mut txn = self.database.begin().await?;

		let player_id = match req.player {
			None => None,
			Some(player) => Some(player.resolve_id(txn.as_mut()).await?),
		};

		let server_id = match req.server {
			None => None,
			Some(server) => Some(server.resolve_id(txn.as_mut()).await?),
		};

		let jumpstats = sqlx::query_as::<_, FetchJumpstatResponse>(&format!(
			r"
			{}
			WHERE
			  j.type = COALESCE(?, j.type)
			  AND j.mode = COALESCE(?, j.mode)
			  AND j.distance = COALESCE(?, j.distance)
			  AND p.id = COALESCE(?, p.id)
			  AND s.id = COALESCE(?, s.id)
			  AND j.created_on > COALESCE(?, '1970-01-01 00:00:01')
			  AND j.created_on < COALESCE(?, '2038-01-19 03:14:07')
			LIMIT
			  ? OFFSET ?
			",
			queries::SELECT,
		))
		.bind(req.jump_type)
		.bind(req.mode)
		.bind(req.minimum_distance)
		.bind(player_id)
		.bind(server_id)
		.bind(req.created_after)
		.bind(req.created_before)
		.bind(*req.limit)
		.bind(*req.offset)
		.fetch_all(txn.as_mut())
		.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchJumpstatsResponse { jumpstats, total })
	}

	/// Submit a new jumpstat.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn submit_jumpstat(
		&self,
		req: SubmitJumpstatRequest,
	) -> Result<SubmitJumpstatResponse>
	{
		let mut txn = self.database.begin().await?;

		let jumpstat_id = sqlx::query! {
			r#"
			INSERT INTO
			  Jumpstats (
			    type,
			    mode,
			    strafes,
			    distance,
			    sync,
			    pre,
			    max,
			    overlap,
			    bad_angles,
			    dead_air,
			    height,
			    airpath,
			    deviation,
			    average_width,
			    airtime,
			    player_id,
			    server_id,
			    plugin_version_id
			  )
			VALUES
			  (
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?
			  )
			"#,
			req.jump_type,
			req.mode,
			req.strafes,
			req.distance,
			req.sync,
			req.pre,
			req.max,
			req.overlap,
			req.bad_angles,
			req.dead_air,
			req.height,
			req.airpath,
			req.deviation,
			req.average_width,
			req.airtime.as_secs_f64(),
			req.player_id,
			req.server_id,
			req.server_plugin_version_id,
		}
		.execute(txn.as_mut())
		.await
		.map_err(|error| {
			if error.is_fk_violation("player_id") {
				Error::PlayerDoesNotExist { steam_id: req.player_id }
			} else {
				Error::Database(error)
			}
		})?
		.last_insert_id()
		.conv::<JumpstatID>();

		txn.commit().await?;

		tracing::trace!(%jumpstat_id, "submitted new jumpstat");

		Ok(SubmitJumpstatResponse { jumpstat_id })
	}
}
