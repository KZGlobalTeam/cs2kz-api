use axum::Router;

use crate::State;

pub mod routes;

pub fn router(state: &'static State) -> Router {
	Router::new().with_state(state)
}
