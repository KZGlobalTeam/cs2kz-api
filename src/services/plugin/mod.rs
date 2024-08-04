//! A service for managing the CS2KZ plugin.

use std::fmt;

use axum::extract::FromRef;
use sqlx::{MySql, Pool};
use tap::TryConv;

use crate::database::TransactionExt;

pub(crate) mod http;

mod error;
pub use error::{Error, Result};

pub(crate) mod models;
pub use models::{
	FetchPluginVersionRequest,
	FetchPluginVersionResponse,
	FetchPluginVersionsRequest,
	FetchPluginVersionsResponse,
	PluginVersion,
	PluginVersionID,
	PluginVersionIdentifier,
	SubmitPluginVersionRequest,
	SubmitPluginVersionResponse,
};

/// A service for managing KZ maps.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct PluginService
{
	database: Pool<MySql>,
}

impl fmt::Debug for PluginService
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("PluginService").finish_non_exhaustive()
	}
}

impl PluginService
{
	/// Create a new [`PluginService`].
	#[tracing::instrument]
	pub fn new(database: Pool<MySql>) -> Self
	{
		Self { database }
	}

	/// Fetch a plugin version by its ID, semver version, or git revision.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_version(
		&self,
		req: FetchPluginVersionRequest,
	) -> Result<Option<FetchPluginVersionResponse>>
	{
		let res = sqlx::query_as(
			r"
			SELECT
			  id,
			  semver,
			  git_revision,
			  created_on
			FROM
			  PluginVersions
			WHERE
			  id = COALESCE(?, id)
			  AND semver = COALESCE(?, semver)
			  AND git_revision = COALESCE(?, git_revision)
			LIMIT
			  1
			",
		)
		.bind(req.ident.as_id())
		.bind(req.ident.as_semver())
		.bind(req.ident.as_git_rev())
		.fetch_optional(&self.database)
		.await?;

		Ok(res)
	}

	/// Fetch plugin versions.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_versions(
		&self,
		req: FetchPluginVersionsRequest,
	) -> Result<FetchPluginVersionsResponse>
	{
		let mut txn = self.database.begin().await?;
		let versions = sqlx::query_as(
			r"
			SELECT
			  SQL_CALC_FOUND_ROWS id,
			  semver,
			  git_revision,
			  created_on
			FROM
			  PluginVersions
			LIMIT
			  ? OFFSET ?
			",
		)
		.bind(*req.limit)
		.bind(*req.offset)
		.fetch_all(txn.as_mut())
		.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchPluginVersionsResponse { versions, total })
	}

	/// Submit a new plugin version.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn submit_version(
		&self,
		req: SubmitPluginVersionRequest,
	) -> Result<SubmitPluginVersionResponse>
	{
		let mut txn = self.database.begin().await?;

		let latest_version = sqlx::query! {
			r"
			SELECT
			  semver `semver: PluginVersion`
			FROM
			  PluginVersions
			ORDER BY
			  created_on DESC
			LIMIT
			  1
			",
		}
		.fetch_optional(txn.as_mut())
		.await?
		.map(|row| row.semver);

		if let Some(latest) = latest_version.filter(|v| v >= &req.semver) {
			tracing::warn! {
				target: "cs2kz_api::audit_log",
				%latest,
				%req.semver,
				"submitted outdated plugin version",
			};

			return Err(Error::OutdatedVersion { latest, actual: req.semver });
		}

		let plugin_version_id = sqlx::query! {
			r"
			INSERT INTO
			  PluginVersions (semver, git_revision)
			VALUES
			  (?, ?)
			",
			req.semver,
			req.git_revision,
		}
		.execute(txn.as_mut())
		.await?
		.last_insert_id()
		.try_conv::<PluginVersionID>()
		.expect("in-range ID");

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			version = %req.semver,
			revision = req.git_revision,
			"registered new plugin version",
		};

		Ok(SubmitPluginVersionResponse { plugin_version_id })
	}
}
