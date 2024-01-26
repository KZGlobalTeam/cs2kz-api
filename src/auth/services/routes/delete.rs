use axum::extract::Path;

use crate::extract::State;
use crate::{audit, responses, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  tag = "Services",
  path = "/services/{id}",
  params(("id" = u64, Path, description = "The service ID")),
  responses(
    responses::Ok<()>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["admin"]),
  ),
)]
pub async fn delete(state: State, Path(service_id): Path<u64>) -> Result<()> {
	let result = sqlx::query! {
		r#"
		DELETE FROM
		  Services
		WHERE
		  id = ?
		"#,
		service_id,
	}
	.execute(state.database())
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::InvalidServiceID(service_id));
	}

	audit!("deleted service", id = %service_id);

	Ok(())
}
