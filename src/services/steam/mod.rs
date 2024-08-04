//! The [`SteamService`] is responsible for communicating with Steam.
//!
//! It can provide profile information about users, generate OpenID URLs, and
//! verify their payloads.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::FromRef;
use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use url::Url;

mod error;
pub use error::{Error, Result};

pub mod openid;
pub use openid::OpenIDPayload;

pub mod user;
pub use user::User;

pub mod workshop;
pub use workshop::WorkshopID;

/// Steam Web API URL for fetching user information.
const USER_URL: &str = "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002";

/// Steam Web API URL for fetching map information.
const MAP_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

/// A service for interacting with Steam.
#[derive(Clone)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct SteamService
{
	api_url: Arc<Url>,
	steam_api_key: Arc<str>,

	#[cfg(feature = "production")]
	workshop_artifacts_path: Arc<Path>,

	#[cfg(not(feature = "production"))]
	workshop_artifacts_path: Option<Arc<Path>>,

	#[cfg(feature = "production")]
	depot_downloader_path: Arc<Path>,

	#[cfg(not(feature = "production"))]
	depot_downloader_path: Option<Arc<Path>>,

	http_client: reqwest::Client,
}

impl fmt::Debug for SteamService
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("SteamService")
			.field("api_url", &format_args!("{:?}", self.api_url.as_str()))
			.field("workshop_artifacts_path", &self.workshop_artifacts_path)
			.field("depot_downloader_path", &self.depot_downloader_path)
			.finish_non_exhaustive()
	}
}

impl FromRef<SteamService> for reqwest::Client
{
	fn from_ref(svc: &SteamService) -> Self
	{
		svc.http_client.clone()
	}
}

impl SteamService
{
	/// Creates a new [`SteamService`].
	pub fn new(
		api_url: Arc<Url>,
		steam_api_key: String,
		#[cfg(feature = "production")] workshop_artifacts_path: PathBuf,
		#[cfg(not(feature = "production"))] workshop_artifacts_path: Option<PathBuf>,
		#[cfg(feature = "production")] depot_downloader_path: PathBuf,
		#[cfg(not(feature = "production"))] depot_downloader_path: Option<PathBuf>,
		http_client: reqwest::Client,
	) -> Self
	{
		Self {
			api_url,
			steam_api_key: steam_api_key.into(),

			#[cfg(feature = "production")]
			workshop_artifacts_path: workshop_artifacts_path.into(),

			#[cfg(not(feature = "production"))]
			workshop_artifacts_path: workshop_artifacts_path.map(Into::into),

			#[cfg(feature = "production")]
			depot_downloader_path: depot_downloader_path.into(),

			#[cfg(not(feature = "production"))]
			depot_downloader_path: depot_downloader_path.map(Into::into),

			http_client,
		}
	}

	/// Builds OpenID form parameters to send to Steam.
	#[tracing::instrument(level = "debug")]
	pub fn openid_login_form(&self) -> openid::LoginForm
	{
		openid::LoginForm::new(Url::clone(&*self.api_url))
	}

	/// Fetch information about a user.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_user(&self, user_id: SteamID) -> Result<User>
	{
		#[allow(clippy::missing_docs_in_private_items)]
		#[derive(Serialize)]
		struct Query<'a>
		{
			key: &'a str,

			#[serde(rename = "steamids", serialize_with = "SteamID::serialize_u64")]
			user_id: SteamID,
		}

		tracing::debug!(url = USER_URL, "making http request to steam");

		let response = self
			.http_client
			.get(USER_URL)
			.query(&Query { key: &self.steam_api_key, user_id })
			.send()
			.await?;

		if let Err(error) = response.error_for_status_ref() {
			let response_body = response.text().await.ok();

			tracing::error! {
				?error,
				?response_body,
				"failed to fetch profile information from steam",
			};

			return Err(Error::Http(error));
		}

		let user = response.json::<User>().await?;

		Ok(user)
	}

	/// Fetches a map's name from the workshop.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_map_name(&self, workshop_id: WorkshopID) -> Result<String>
	{
		#[allow(clippy::missing_docs_in_private_items)]
		struct Params
		{
			workshop_id: WorkshopID,
		}

		impl Serialize for Params
		{
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

		#[allow(clippy::missing_docs_in_private_items)]
		struct MapInfo
		{
			title: String,
		}

		impl<'de> Deserialize<'de> for MapInfo
		{
			#[allow(clippy::missing_docs_in_private_items)]
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				#[derive(Deserialize)]
				struct Helper1
				{
					response: Helper2,
				}

				#[derive(Deserialize)]
				struct Helper2
				{
					publishedfiledetails: Vec<serde_json::Value>,
				}

				let title = Helper1::deserialize(deserializer)
					.map(|x| x.response)
					.map(|mut x| x.publishedfiledetails.remove(0))
					.map(|mut json| {
						json.get_mut("title")
							.unwrap_or(&mut serde_json::Value::Null)
							.take()
					})?;

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
					serde_json::Value::String(title) => Ok(Self { title }),
					serde_json::Value::Null => Err(serde::de::Error::missing_field("title")),
					serde_json::Value::Bool(v) => invalid_type!(Bool(v)),
					serde_json::Value::Number(v) => {
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
					serde_json::Value::Array(_) => invalid_type!(Seq),
					serde_json::Value::Object(_) => invalid_type!(Map),
				}
			}
		}

		tracing::debug!(url = MAP_URL, "making http request to steam");

		let response = self
			.http_client
			.post(MAP_URL)
			.form(&Params { workshop_id })
			.send()
			.await?;

		if !response.status().is_success() {
			return Err(Error::InvalidWorkshopID { workshop_id });
		}

		let name = response
			.json::<MapInfo>()
			.await
			.map(|info| info.title)
			.inspect_err(|error| tracing::debug!(%error, "failed to deserialize workshop map"))
			.map_err(|_| Error::NotAMap { workshop_id })?;

		Ok(name)
	}

	/// Downloads a map from the workshop.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn download_map(&self, workshop_id: WorkshopID) -> Result<workshop::MapFile>
	{
		#[cfg(feature = "production")]
		let (workshop_artifacts_path, depot_downloader_path) =
			(&*self.workshop_artifacts_path, &*self.depot_downloader_path);

		#[cfg(not(feature = "production"))]
		let (workshop_artifacts_path, depot_downloader_path) =
			(self.workshop_artifacts_path.as_deref(), self.depot_downloader_path.as_deref());

		workshop::MapFile::download(workshop_id, workshop_artifacts_path, depot_downloader_path)
			.await
			.map_err(Error::DownloadWorkshopMap)
	}
}
