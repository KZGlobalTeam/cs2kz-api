use std::path::Path;
use std::result::Result as StdResult;
use std::sync::Arc;

use reqwest::header;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::error;

use super::{Error, Result};

/// A Steam Workshop Map.
pub struct Map {
	pub name: String,
}

impl Map {
	pub const URL: &'static str =
		"https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

	/// Fetches the Workshop Map with ID `id` from Steam using the provided `http_client`.
	pub async fn get(id: u32, http_client: Arc<reqwest::Client>) -> Result<Self> {
		let params = serde_urlencoded::to_string(GetMapParams { id }).expect("this is valid");

		let response = http_client
			.post(Self::URL)
			.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
			.body(params)
			.send()
			.await?;

		if !response.status().is_success() {
			return Err(Error::InvalidWorkshopID(id));
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
	#[rustfmt::skip]
	const DOWNLOAD_COMMAND: &'static [&'static str] = &[
		"+force_install_dir", "/kz/workshop",
		"+login", "anonymous",
		"+workshop_download_item", "730",
	];

	const DOWNLOAD_DIR: &'static str = "/kz/workshop/steamapps/workshop/content/730";

	/// Downloads the workshop map with the given ID and returns a handle to the file.
	///
	/// NOTE: This shells out to steamcmd, so it might take a few seconds.
	pub async fn download(workshop_id: u32) -> Result<Self> {
		let args = Self::DOWNLOAD_COMMAND
			.iter()
			.copied()
			.map(String::from)
			.chain([workshop_id.to_string(), String::from("+quit")]);

		let output = Command::new("/bin/steamcmd")
			.args(args)
			.spawn()
			.map_err(|err| {
				error!(%err, "failed to run steamcmd");
				Error::SteamCMD(Some(err))
			})?
			.wait_with_output()
			.await
			.map_err(|err| {
				error!(%err, "failed to wait for steamcmd");
				Error::SteamCMD(Some(err))
			})?;

		if let Err(err) = io::stderr().flush().await {
			error!(%err, "failed to flush stderr");
		}

		if !output.status.success() {
			error!(?output, "steamcmd exited abnormally");
			return Err(Error::SteamCMD(None));
		}

		let path = Path::new(Self::DOWNLOAD_DIR).join(format!("{workshop_id}/{workshop_id}.vpk"));
		let file = File::open(path).await.map_err(|err| {
			error!(%err, "failed to open workshop map file");
			Error::IO(err)
		})?;

		Ok(Self { file })
	}

	/// Calculates the checksum for this file using [crc32].
	///
	/// [crc32]: crc32fast
	pub async fn checksum(&mut self) -> Result<u32> {
		let mut buf = Vec::new();
		self.file.read_to_end(&mut buf).await.map_err(|err| {
			error!(%err, "failed to read workshop map file");
			Error::IO(err)
		})?;

		Ok(crc32fast::hash(&buf))
	}
}
