//! Everything related to the [CS2KZ plugin].
//!
//! [CS2KZ plugin]: https://github.com/KZGlobalTeam/cs2kz-metamod

use axum::{routing, Router};

use crate::middleware::cors;
use crate::State;

mod models;
pub use models::{CreatedPluginVersion, NewPluginVersion, PluginVersion, PluginVersionID};

pub mod handlers;

/// Returns an [`axum::Router`] for the `/plugin` routes.
pub fn router(state: State) -> Router {
	Router::new()
		.route("/versions", routing::get(handlers::versions::get))
		.route_layer(cors::permissive())
		.route("/versions", routing::post(handlers::versions::post))
		.with_state(state.clone())
}
