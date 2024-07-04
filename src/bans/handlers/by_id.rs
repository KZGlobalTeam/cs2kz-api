//! HTTP handlers for the `/bans/{ban_id}` routes.

use axum::extract::Path;
use axum::Json;
use sqlx::{MySqlExecutor, QueryBuilder};

use crate::authorization::{self, Permissions};
use crate::bans::{queries, Ban, BanID, BanUpdate, CreatedUnban, NewUnban, UnbanID};
use crate::openapi::responses;
use crate::openapi::responses::{Created, NoContent};
use crate::sqlx::UpdateQuery;
use crate::{authentication, Error, Result, State};

/// Fetch a specific ban by its ID.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/bans/{ban_id}",
  tag = "Bans",
  params(("ban_id" = u64, Path, description = "The ban's ID")),
  responses(
    responses::Ok<Ban>,
    responses::NotFound,
    responses::BadRequest,
  ),
)]
pub async fn get(state: State, Path(ban_id): Path<BanID>) -> Result<Json<Ban>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE b.id = ").push_bind(ban_id);

	let ban = query
		.build_query_as::<Ban>()
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| Error::not_found("ban"))?;

	Ok(Json(ban))
}

/// Update an existing ban.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  path = "/bans/{ban_id}",
  tag = "Bans",
  security(("Browser Session" = ["bans"])),
  params(("ban_id" = u64, Path, description = "The ban's ID")),
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
  ),
)]
pub async fn patch(
	state: State,
	session: authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	Path(ban_id): Path<BanID>,
	Json(BanUpdate { reason, expires_on }): Json<BanUpdate>,
) -> Result<NoContent> {
	if reason.is_none() && expires_on.is_none() {
		return Ok(NoContent);
	}

	let mut transaction = state.transaction().await?;

	if let Some(unban_id) = is_already_unbanned(ban_id, transaction.as_mut()).await? {
		return Err(Error::ban_already_reverted(ban_id, unban_id));
	}

	let mut query = UpdateQuery::new("Bans");

	if let Some(reason) = reason {
		query.set(" reason ", reason);
	}

	if let Some(expires_on) = expires_on {
		query.set(" expires_on ", expires_on);
	}

	query.push(" WHERE id = ").push_bind(ban_id);

	let query_result = query.build().execute(transaction.as_mut()).await?;

	match query_result.rows_affected() {
		0 => return Err(Error::not_found("ban")),
		n => assert_eq!(n, 1, "updated more than 1 ban"),
	}

	transaction.commit().await?;

	tracing::info!(target: "cs2kz_api::audit_log", %ban_id, "updated ban");

	Ok(NoContent)
}

/// Revert a ban.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  path = "/bans/{ban_id}",
  tag = "Bans",
  security(("Browser Session" = ["bans"])),
  params(("ban_id" = u64, Path, description = "The ban's ID")),
  responses(
    responses::Created<CreatedUnban>,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::Conflict,
  ),
)]
pub async fn delete(
	state: State,
	session: authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	Path(ban_id): Path<BanID>,
	Json(NewUnban { reason }): Json<NewUnban>,
) -> Result<Created<Json<CreatedUnban>>> {
	let mut transaction = state.transaction().await?;

	if let Some(unban_id) = is_already_unbanned(ban_id, transaction.as_mut()).await? {
		return Err(Error::ban_already_reverted(ban_id, unban_id));
	}

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Bans
		SET
		  expires_on = NOW()
		WHERE
		  id = ?
		"#,
		ban_id,
	}
	.execute(transaction.as_mut())
	.await?;

	match query_result.rows_affected() {
		0 => return Err(Error::not_found("ban")),
		n => assert_eq!(n, 1, "updated more than 1  ban"),
	}

	tracing::info!(target: "cs2kz_api::audit_log", %ban_id, "reverted ban");

	let unban_id = sqlx::query! {
		r#"
		INSERT INTO
		  Unbans (ban_id, reason, admin_id)
		VALUES
		  (?, ?, ?)
		"#,
		ban_id,
		reason,
		session.user().steam_id(),
	}
	.execute(transaction.as_mut())
	.await?
	.last_insert_id()
	.into();

	transaction.commit().await?;

	tracing::info!(target: "cs2kz_api::audit_log", %ban_id, %unban_id, "created unban");

	Ok(Created(Json(CreatedUnban { unban_id })))
}

/// Checks if a ban has already been reverted, and returns the corresponding [`UnbanID`].
async fn is_already_unbanned(
	ban_id: BanID,
	executor: impl MySqlExecutor<'_>,
) -> Result<Option<UnbanID>> {
	sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: UnbanID`
		FROM
		  Unbans
		WHERE
		  ban_id = ?
		"#,
		ban_id,
	}
	.fetch_optional(executor)
	.await
	.map_err(Error::from)
}
