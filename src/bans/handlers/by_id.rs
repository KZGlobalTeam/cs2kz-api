//! Handlers for the `/bans/{ban_id}` route.

use std::num::NonZeroU64;

use axum::extract::Path;
use axum::Json;
use sqlx::{MySqlExecutor, QueryBuilder};
use tracing::info;

use crate::auth::RoleFlags;
use crate::bans::{queries, Ban, BanUpdate, CreatedUnban, NewUnban};
use crate::responses::{Created, NoContent};
use crate::sqlx::UpdateQuery;
use crate::{auth, responses, AppState, Error, Result};

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/bans/{ban_id}",
  tag = "Bans",
  params(("ban_id" = u64, Path, description = "The ban's ID")),
  responses(
    responses::Ok<Ban>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: AppState, Path(ban_id): Path<NonZeroU64>) -> Result<Json<Ban>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE b.id = ").push_bind(ban_id.get());

	let ban = query
		.build_query_as::<Ban>()
		.fetch_optional(&state.database)
		.await?
		.ok_or(Error::no_content())?;

	Ok(Json(ban))
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  patch,
  path = "/bans/{ban_id}",
  tag = "Bans",
  security(("Browser Session" = ["bans"])),
  params(("ban_id" = u64, Path, description = "The ban's ID")),
  responses(//
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn patch(
	state: AppState,
	session: auth::Session<auth::HasRoles<{ RoleFlags::BANS.as_u32() }>>,
	Path(ban_id): Path<NonZeroU64>,
	Json(BanUpdate { reason, expires_on }): Json<BanUpdate>,
) -> Result<NoContent> {
	if reason.is_none() && expires_on.is_none() {
		return Ok(NoContent);
	}

	if let Some(ban_id) = is_already_unbanned(ban_id, &state.database).await? {
		return Err(Error::ban_already_reverted(ban_id));
	}

	let mut query = UpdateQuery::new("UPDATE Bans");

	if let Some(reason) = reason {
		query.set(" reason ", reason);
	}

	if let Some(expires_on) = expires_on {
		query.set(" expires_on ", expires_on);
	}

	query.push(" WHERE id = ").push_bind(ban_id.get());

	let query_result = query.build().execute(&state.database).await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("ban ID"));
	}

	info!(target: "audit_log", %ban_id, "updated ban");

	Ok(NoContent)
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  delete,
  path = "/bans/{ban_id}",
  tag = "Bans",
  security(("Browser Session" = ["bans"])),
  params(("ban_id" = u64, Path, description = "The ban's ID")),
  responses(
    responses::Created<CreatedUnban>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::InternalServerError,
  ),
)]
pub async fn delete(
	state: AppState,
	session: auth::Session<auth::HasRoles<{ RoleFlags::BANS.as_u32() }>>,
	Path(ban_id): Path<NonZeroU64>,
	Json(NewUnban { reason }): Json<NewUnban>,
) -> Result<Created<Json<CreatedUnban>>> {
	let mut transaction = state.database.begin().await?;

	if let Some(ban_id) = is_already_unbanned(ban_id, transaction.as_mut()).await? {
		return Err(Error::ban_already_reverted(ban_id));
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
		ban_id.get(),
	}
	.execute(transaction.as_mut())
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("ban ID"));
	}

	info!(target: "audit_log", %ban_id, "reverted ban");

	let unban_id = sqlx::query! {
		r#"
		INSERT INTO
		  Unbans (ban_id, reason, admin_id)
		VALUES
		  (?, ?, ?)
		"#,
		ban_id.get(),
		reason,
		session.user().steam_id(),
	}
	.execute(transaction.as_mut())
	.await
	.map(crate::sqlx::last_insert_id)??;

	info!(target: "audit_log", %ban_id, %unban_id, "created unban");

	transaction.commit().await?;

	Ok(Created(Json(CreatedUnban { unban_id })))
}

/// Checks if there is an unban associated with the given `ban_id` and returns the corresponding
/// `ban_id`.
async fn is_already_unbanned(
	ban_id: NonZeroU64,
	executor: impl MySqlExecutor<'_>,
) -> Result<Option<NonZeroU64>> {
	sqlx::query! {
		r#"
		SELECT
		  id
		FROM
		  Unbans
		WHERE
		  ban_id = ?
		"#,
		ban_id.get(),
	}
	.fetch_optional(executor)
	.await?
	.map(|row| {
		NonZeroU64::new(row.id).ok_or_else(|| Error::internal_server_error("unban ID was 0"))
	})
	.transpose()
}
