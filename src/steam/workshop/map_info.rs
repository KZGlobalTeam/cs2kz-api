//! Functions for fetching information about Workshop Maps.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
	#[allow(clippy::missing_docs_in_private_items)]
	struct Params {
		workshop_id: WorkshopID,
	}

	impl Serialize for Params {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			use serde::ser::SerializeStruct;

			let mut serializer = serializer.serialize_struct("params", 2)?;

			serializer.serialize_field("itemcount", &1)?;
			serializer.serialize_field("publishedfileids[0]", &self.workshop_id)?;
			serializer.end()
		}
	}

	let response = http_client
		.post(API_URL)
		.form(&Params { workshop_id })
		.send()
		.await?;

	if !response.status().is_success() {
		return Err(Error::not_found("workshop map"));
	}

	let map_info = response
		.json::<MapInfo>()
		.await
		.inspect_err(|error| tracing::debug!(%error, "failed to deserialize workshop map"))
		.map_err(|_| Error::not_a_map(workshop_id))?;

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
		#[derive(Deserialize)]
		struct Helper1 {
			response: Helper2,
		}

		#[derive(Deserialize)]
		struct Helper2 {
			publishedfiledetails: Vec<JsonValue>,
		}

		let title = Helper1::deserialize(deserializer)
			.map(|x| x.response)
			.map(|mut x| x.publishedfiledetails.remove(0))
			.map(|mut json| json.get_mut("title").unwrap_or(&mut JsonValue::Null).take())?;

		macro_rules! invalid_type {
			($unexpected:ident) => {
				Err(serde::de::Error::invalid_type(
					serde::de::Unexpected::$unexpected,
					&"string",
				))
			};
			($unexpected:ident($v:expr)) => {
				Err(serde::de::Error::invalid_type(
					serde::de::Unexpected::$unexpected($v),
					&"string",
				))
			};
		}

		match title {
			JsonValue::String(title) => Ok(Self { title }),
			JsonValue::Null => Err(serde::de::Error::missing_field("title")),
			JsonValue::Bool(v) => invalid_type!(Bool(v)),
			JsonValue::Number(v) => {
				if let Some(v) = v.as_i64() {
					invalid_type!(Signed(v))
				} else if let Some(v) = v.as_u64() {
					invalid_type!(Unsigned(v))
				} else if let Some(v) = v.as_f64() {
					invalid_type!(Float(v))
				} else {
					unreachable!()
				}
			}
			JsonValue::Array(_) => invalid_type!(Seq),
			JsonValue::Object(_) => invalid_type!(Map),
		}
	}
}
