use axum::Json;

use crate::auth::services::models::CreatedService;
use crate::auth::services::NewService;
use crate::auth::Service;
use crate::extract::State;
use crate::responses::Created;
use crate::{responses, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  tag = "Services",
  path = "/services",
  request_body = NewService,
  responses(
    responses::Created<CreatedService>,
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["admin"]),
  ),
)]
pub async fn create(
	state: State,
	Json(NewService { name, roles }): Json<NewService>,
) -> Result<Created<Json<CreatedService>>> {
	let role_flags = roles.into_iter().collect();
	let mut transaction = state.transaction().await?;
	let service = Service::<0>::new(name, role_flags, &mut transaction).await?;

	transaction.commit().await?;

	Ok(Created(Json(CreatedService {
		service_id: service.id,
		service_key: service.key,
	})))
}
