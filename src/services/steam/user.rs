//! This module contains the [`User`] type, which represents a user's profile
//! information.

use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

/// HTTP cookie name for storing a serialized [`User`].
pub const COOKIE_NAME: &str = "kz-player";

/// A Steam user.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(example = json!({
  "steam_id": "STEAM_1:1:161178172",
  "steam_id64": "76561198282622073",
  "username": "AlphaKeks",
  "realname": "STEAM_1:1:161178172",
  "country": "DE",
  "profile_url": "https://steamcommunity.com/id/AlphaKeks/",
  "avatar_url": "https://avatars.steamstatic.com/da7587d32ed9cd619be8ecec623ce68a1a0afd63.jpg"
}))]
pub struct User
{
	/// The user's SteamID.
	pub steam_id: SteamID,

	/// The user's SteamID in its stringified 64-bit format.
	#[serde(serialize_with = "SteamID::serialize_u64_stringified")]
	pub steam_id64: SteamID,

	/// The user's username.
	pub username: String,

	/// The user's realname.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub realname: Option<String>,

	/// The user's country.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country: Option<String>,

	/// URL to the user's Steam profile.
	pub profile_url: Url,

	/// URL to the user's Steam avatar.
	pub avatar_url: Url,
}

impl User
{
	/// Serializes this [`User`] as JSON and creates an HTTP cookie containing
	/// that payload.
	pub fn to_cookie(&self, domain: impl Into<String>) -> Cookie<'static>
	{
		let json = serde_json::to_string(self).expect("user should be serializable");

		Cookie::build((COOKIE_NAME, json))
			.domain(domain.into())
			.path("/")
			.secure(cfg!(feature = "production"))
			.http_only(false)
			.permanent()
			.build()
	}
}

impl<'de> Deserialize<'de> for User
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
			players: [Helper3; 1],
		}

		#[derive(Deserialize)]
		struct Helper3
		{
			steamid: SteamID,
			personaname: String,
			realname: Option<String>,
			loccountrycode: Option<String>,
			profileurl: Url,
			avatar: Url,
		}

		Helper1::deserialize(deserializer).map(|x| x.response).map(
			|Helper2 { players: [player] }| Self {
				steam_id: player.steamid,
				steam_id64: player.steamid,
				username: player.personaname,
				realname: player.realname,
				country: player.loccountrycode,
				profile_url: player.profileurl,
				avatar_url: player.avatar,
			},
		)
	}
}
