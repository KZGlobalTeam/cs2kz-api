use axum::extract::Path;
use axum::Json;

use crate::auth::services::models::{CreatedService, ServiceKey};
use crate::extract::State;
use crate::responses::Created;
use crate::{audit, responses, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  tag = "Services",
  path = "/services/{id}/key",
  params(("id" = u64, Path, description = "The service ID")),
  request_body = ServiceUpdate,
  responses(
    responses::Created<CreatedService>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["admin"]),
  ),
)]
pub async fn update_key(
	state: State,
	Path(service_id): Path<u64>,
) -> Result<Created<Json<CreatedService>>> {
	let new_key = ServiceKey::new();

	let result = sqlx::query! {
		r#"
		UPDATE
		  Services
		SET
		  `key` = ?
		WHERE
		  id = ?
		"#,
		new_key,
		service_id,
	}
	.execute(state.database())
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::InvalidServiceID(service_id));
	}

	audit!("updated key for service", id = %service_id);

	Ok(Created(Json(CreatedService { service_id, service_key: new_key })))
}
