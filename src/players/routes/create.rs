use axum::Json;

use crate::auth::{Jwt, Server};
use crate::players::NewPlayer;
use crate::responses::Created;
use crate::sqlx::SqlErrorExt;
use crate::{audit, responses, AppState, Error, Result};

/// Register a new player.
///
/// It is only usable by CS2 servers and will return an error for existing players.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  tag = "Players",
  path = "/players",
  request_body = NewPlayer,
  responses(
    responses::Created<()>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("CS2 Server JWT" = []),
  ),
)]
pub async fn create(
	state: AppState,
	server: Jwt<Server>,
	Json(player): Json<NewPlayer>,
) -> Result<Created<()>> {
	sqlx::query! {
		r#"
		INSERT INTO
		  Players (steam_id, name, last_known_ip_address)
		VALUES
		  (?, ?, ?)
		"#,
		player.steam_id,
		player.name,
		player.ip_address.to_string(),
	}
	.execute(&state.database)
	.await
	.map_err(|err| {
		if err.is_foreign_key_violation() {
			Error::already_exists("player")
		} else {
			Error::from(err)
		}
	})?;

	audit!("created player", steam_id = %player.steam_id, name = %player.name);

	Ok(Created(()))
}
