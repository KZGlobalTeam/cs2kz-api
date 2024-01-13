use axum::extract::Path;
use axum::Json;
use cs2kz::ServerIdentifier;
use sqlx::QueryBuilder;

use crate::extractors::State;
use crate::servers::{queries, Server};
use crate::{responses, Error, Result};

/// Fetch a single server by ID or name.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Servers",
  path = "/servers/{server}",
  params(ServerIdentifier<'_>),
  responses(
    responses::Ok<Server>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_single(
	state: State,
	Path(server): Path<ServerIdentifier<'_>>,
) -> Result<Json<Server>> {
	let mut query = QueryBuilder::new(queries::BASE_SELECT);

	query.push(" WHERE ");

	match server {
		ServerIdentifier::ID(id) => {
			query.push(" s.id = ").push_bind(id);
		}
		ServerIdentifier::Name(name) => {
			query.push(" s.name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	query
		.build_query_as::<Server>()
		.fetch_optional(state.database())
		.await?
		.map(Json)
		.ok_or(Error::NoContent)
}
