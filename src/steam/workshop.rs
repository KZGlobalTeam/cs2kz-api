use std::path::Path;
use std::result::Result as StdResult;

use reqwest::header;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::error;

use crate::{audit, Error, Result};

/// A Steam Workshop Map.
pub struct Map {
	pub name: String,
}

impl Map {
	pub const URL: &'static str =
		"https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

	/// Fetches the Workshop Map with ID `id` from Steam using the provided `http_client`.
	pub async fn get(id: u32, http_client: &reqwest::Client) -> Result<Self> {
		let params = serde_urlencoded::to_string(GetMapParams { id }).expect("this is valid");

		let response = http_client
			.post(Self::URL)
			.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
			.body(params)
			.send()
			.await?;

		if !response.status().is_success() {
			return Err(Error::invalid("workshop ID").with_detail(id));
		}

		response.json().await.map_err(Error::from)
	}
}

impl<'de> Deserialize<'de> for Map {
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
			.map(|mut json| json["title"].take())
			.map(|json| json.as_str().map(ToOwned::to_owned))?
			.map(|name| Self { name })
			.ok_or_else(|| E::missing_field("title"))
	}
}

struct GetMapParams {
	id: u32,
}

impl Serialize for GetMapParams {
	fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serializer = serializer.serialize_struct("params", 2)?;
		serializer.serialize_field("itemcount", &1)?;
		serializer.serialize_field("publishedfileids[0]", &self.id)?;
		serializer.end()
	}
}

/// A `.vpk` file of a downloaded [Workshop Map].
///
/// [Workshop Map]: Map
pub struct MapFile {
	/// A handle to the file.
	file: File,
}

impl MapFile {
	/// Downloads the workshop map with the given ID and returns a handle to the file.
	///
	/// NOTE: This shells out to DepotDownloader, so it might take a few seconds.
	pub async fn download(workshop_id: u32, config: &crate::Config) -> Result<Self> {
		let steam_workshop_path = config
			.steam
			.steam_workshop_path
			.as_deref()
			.map(Path::to_string_lossy)
			.ok_or_else(|| {
				let error = Error::download_workshop_map();

				if cfg!(feature = "production") {
					return error;
				}

				error.with_detail("missing workshop asset directory")
			})?;

		let downloader = config
			.steam
			.workshop_downloader_path
			.as_deref()
			.ok_or_else(|| {
				let error = Error::download_workshop_map();

				if cfg!(feature = "production") {
					return error;
				}

				error.with_detail("missing path to DepotDownloader executable")
			})?;

		let output = Command::new(downloader)
			.args(["-app", "730", "-pubfile"])
			.arg(workshop_id.to_string())
			.args(["-dir", &*steam_workshop_path])
			.spawn()
			.map_err(|err| Error::download_workshop_map().with_detail(err.to_string()))?
			.wait_with_output()
			.await
			.map_err(|err| Error::download_workshop_map().with_detail(err.to_string()))?;

		if let Err(err) = io::stderr().flush().await {
			error!(%err, "failed to flush stderr");
		}

		if !output.status.success() {
			error!(?output, "DepotDownloader exited abnormally");

			let error = Error::download_workshop_map();

			if cfg!(feature = "production") {
				return Err(error);
			}

			return Err(error.with_detail("DepotDownloader exited abnormally"));
		}

		let path = Path::new(&*steam_workshop_path).join(format!("{workshop_id}.vpk"));

		let file = File::open(path).await.map_err(|err| {
			let error = Error::download_workshop_map();

			if cfg!(feature = "production") {
				return error;
			}

			error
				.with_message("failed to open map file")
				.with_detail(err.to_string())
		})?;

		Ok(Self { file })
	}

	/// Calculates the checksum for this file using [crc32].
	///
	/// [crc32]: crc32fast
	pub async fn checksum(&mut self) -> Result<u32> {
		let mut buf = Vec::new();
		self.file.read_to_end(&mut buf).await.map_err(|err| {
			audit!(error, "failed to read map file", %err);

			Error::download_workshop_map()
		})?;

		Ok(crc32fast::hash(&buf))
	}
}
