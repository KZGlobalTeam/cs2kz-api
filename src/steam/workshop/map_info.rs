//! Everything related to Workshop Maps.

use std::result::Result as StdResult;

use reqwest::header;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value as JsonValue;

use crate::steam::workshop::{WorkshopID, API_URL};
use crate::{Error, Result};

/// Fetches the name of the map with the given `workshop_id`.
pub async fn fetch_map_name(
	workshop_id: WorkshopID,
	http_client: &reqwest::Client,
) -> Result<String> {
	#[derive(Serialize)]
	#[allow(clippy::missing_docs_in_private_items)]
	struct Params {
		workshop_id: WorkshopID,
	}

	let query_params =
		serde_urlencoded::to_string(Params { workshop_id }).expect("valid query params");

	let response = http_client
		.post(API_URL)
		.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
		.body(query_params)
		.send()
		.await?;

	if !response.status().is_success() {
		return Err(Error::unknown("workshop ID"));
	}

	let map_info = response.json::<MapInfo>().await?;

	Ok(map_info.title)
}

/// Information about a workshop map.
#[allow(clippy::missing_docs_in_private_items)]
struct MapInfo {
	/// The map's name.
	title: String,
}

impl<'de> Deserialize<'de> for MapInfo {
	#[allow(clippy::missing_docs_in_private_items)]
	fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de::Error as E;

		#[derive(Deserialize)]
		struct Helper1 {
			response: Helper2,
		}

		#[derive(Deserialize)]
		struct Helper2 {
			publishedfiledetails: Vec<JsonValue>,
		}

		Helper1::deserialize(deserializer)
			.map(|x| x.response)
			.map(|mut x| x.publishedfiledetails.remove(0))
			.map(|mut json| json.get_mut("title").unwrap_or(&mut JsonValue::Null).take())
			.map(|json| json.as_str().map(ToOwned::to_owned))?
			.map(|title| Self { title })
			.ok_or_else(|| E::missing_field("title"))
	}
}
