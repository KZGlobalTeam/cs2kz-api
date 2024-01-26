use axum::extract::Path;
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::auth::services::models::ServiceKey;
use crate::auth::{RoleFlags, Service};
use crate::extract::State;
use crate::params::{Limit, Offset};
use crate::{responses, Error, Result};

/// Query Parameters for fetching [`Service`]s.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetServicesParams {
	/// Maximum amount of results.
	pub limit: Limit,

	/// Offset used for pagination.
	pub offset: Offset,
}

#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Services",
  path = "/services/{id}",
  params(("id" = u64, Path, description = "The service ID")),
  responses(
    responses::Ok<Service>,
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["admin"]),
  ),
)]
pub async fn get_single(state: State, Path(service_id): Path<u64>) -> Result<Json<Service>> {
	let service = sqlx::query! {
		r#"
		SELECT
		  id,
		  name,
		  `key` `service_key: ServiceKey`,
		  role_flags `role_flags: RoleFlags`
		FROM
		  Services
		WHERE
		  id = ?
		"#,
		service_id,
	}
	.fetch_optional(state.database())
	.await?
	.map(|row| Service {
		id: row.id,
		name: row.name,
		key: row.service_key,
		role_flags: row.role_flags,
	})
	.ok_or(Error::NoContent)?;

	Ok(Json(service))
}
