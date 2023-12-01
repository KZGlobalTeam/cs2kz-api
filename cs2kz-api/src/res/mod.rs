use {
	axum::{http::StatusCode, response::IntoResponse},
	cs2kz::SteamID,
	serde::{Deserialize, Serialize},
	sqlx::FromRow,
	utoipa::ToSchema,
};

pub mod responses;

pub mod player;
pub mod bans;
pub mod maps;
pub mod servers;
pub mod records;

/// Information about a player.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct PlayerInfo {
	/// The player's Steam name.
	pub name: String,

	/// The player's `SteamID`.
	pub steam_id: SteamID,
}

/// Wraps something such that a generated [`Response`] will have an HTTP status code of 201.
///
/// [`Response`]: axum::response::Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Created<T>(pub T);

impl<T> IntoResponse for Created<T>
where
	T: IntoResponse,
{
	fn into_response(self) -> axum::response::Response {
		(StatusCode::CREATED, self.0).into_response()
	}
}
