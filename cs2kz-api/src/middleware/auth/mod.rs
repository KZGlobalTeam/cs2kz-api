use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use serde::de::DeserializeOwned;

use crate::{Error, Result, State};

pub mod jwt;
pub mod gameservers;
pub mod website;

fn verify_jwt<T, F>(state: State, token: Authorization<Bearer>, expires_at: F) -> Result<T>
where
	T: DeserializeOwned,
	F: FnOnce(&T) -> u64,
{
	let data = state.jwt().decode::<T>(token.token())?.claims;

	if expires_at(&data) < jsonwebtoken::get_current_timestamp() {
		return Err(Error::Unauthorized);
	}

	Ok(data)
}
