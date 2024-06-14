//! Functions for fetching information about Workshop Maps.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value as JsonValue;

use crate::steam::workshop::WorkshopID;
use crate::{Error, Result};

/// Steam Web API URL for fetching map information.
const API_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

/// Fetches the name of a Workshop Map.
#[tracing::instrument(level = "debug", skip(http_client), ret)]
pub async fn fetch_map_name(
	workshop_id: WorkshopID,
	http_client: &reqwest::Client,
) -> Result<String> {
	#[derive(Serialize)]
	#[allow(clippy::missing_docs_in_private_items)]
	struct Params {
		workshop_id: WorkshopID,
	}

	let response = http_client
		.post(API_URL)
		.form(&Params { workshop_id })
		.send()
		.await?;

	if !response.status().is_success() {
		return Err(Error::unknown("workshop ID"));
	}

	let map_info = response.json::<MapInfo>().await?;

	Ok(map_info.title)
}

#[allow(clippy::missing_docs_in_private_items)]
struct MapInfo {
	title: String,
}

impl<'de> Deserialize<'de> for MapInfo {
	#[allow(clippy::missing_docs_in_private_items)]
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
