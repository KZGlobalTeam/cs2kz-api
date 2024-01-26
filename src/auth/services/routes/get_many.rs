use axum::extract::Query;
use axum::Json;
use itertools::Itertools;
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
  path = "/services",
  params(GetServicesParams),
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
pub async fn get_many(
	state: State,
	Query(params): Query<GetServicesParams>,
) -> Result<Json<Vec<Service>>> {
	let services = sqlx::query! {
		r#"
		SELECT
		  id,
		  name,
		  `key` `service_key: ServiceKey`,
		  role_flags `role_flags: RoleFlags`
		FROM
		  Services
		LIMIT
		  ? OFFSET ?
		"#,
		params.limit,
		params.offset,
	}
	.fetch_all(state.database())
	.await?
	.into_iter()
	.map(|row| Service {
		id: row.id,
		name: row.name,
		key: row.service_key,
		role_flags: row.role_flags,
	})
	.collect_vec();

	if services.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(services))
}
