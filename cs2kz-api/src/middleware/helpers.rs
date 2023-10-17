// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::{Error, Result},
	axum::body::Body,
	serde::de::DeserializeOwned,
};

pub async fn deserialize_body<T>(body: Body) -> Result<(T, Body)>
where
	T: DeserializeOwned, {
	let bytes = hyper::body::to_bytes(body).await?;
	let json = serde_json::from_slice::<T>(&bytes).map_err(|_| Error::InvalidRequestBody)?;
	let body = Body::from(bytes);

	Ok((json, body))
}
