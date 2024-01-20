use axum::Json;

use crate::extractors::State;
use crate::responses::Created;
use crate::servers::{CreatedServer, NewServer};
use crate::sqlx::SqlErrorExt;
use crate::{responses, Error, Result};

/// Approve a new KZ server.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  tag = "Servers",
  path = "/servers",
  request_body = NewServer,
  responses(
    responses::Created<CreatedServer>,
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["servers"]),
  ),
)]
pub async fn create(
	state: State,
	Json(server): Json<NewServer>,
) -> Result<Created<Json<CreatedServer>>> {
	let mut transaction = state.transaction().await?;
	let api_key = rand::random::<u32>();

	sqlx::query! {
		r#"
		INSERT INTO
		  Servers (name, ip_address, port, owned_by, api_key)
		VALUES
		  (?, ?, ?, ?, ?)
		"#,
		server.name,
		server.ip_address.ip().to_string(),
		server.ip_address.port(),
		server.owned_by,
		api_key,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_foreign_key_violation() {
			Error::UnknownPlayer { steam_id: server.owned_by }
		} else {
			Error::MySql(err)
		}
	})?;

	let server_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await
		.map(|row| row.id as _)?;

	transaction.commit().await?;

	Ok(Created(Json(CreatedServer { server_id, api_key })))
}
