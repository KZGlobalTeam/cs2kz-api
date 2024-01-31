use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;
use utoipa::ToSchema;

use crate::Result;

/// A player profile.
#[derive(Debug, Serialize, ToSchema)]
pub struct Player {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's SteamID as a stringified 64-bit integer.
	pub steam_id64: String,

	/// The player's public username.
	pub username: String,

	/// The player's "real" name.
	pub realname: Option<String>,

	/// Country code (if the player specified one).
	pub country: Option<String>,

	/// Link to the player's profile.
	#[schema(value_type = String)]
	pub profile_url: Url,

	/// Link to the player's profile picture.
	#[schema(value_type = String)]
	pub avatar_url: Url,
}

impl Player {
	pub const GET_URL: &'static str =
		"http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002";

	pub const COOKIE_NAME: &'static str = "kz-player";

	/// Fetches the player with the given `steam_id` from Steam's API.
	pub async fn fetch(
		steam_id: SteamID,
		api_key: &str,
		http_client: &reqwest::Client,
	) -> Result<Self> {
		let url = Url::parse_with_params(Self::GET_URL, [
			("key", api_key.to_owned()),
			("steamids", steam_id.as_u64().to_string()),
		])
		.expect("this is a valid url");

		let user = http_client.get(url).send().await?.json::<Self>().await?;

		Ok(user)
	}

	/// Creates a cookie containing `self` serialized as JSON.
	pub fn to_cookie(&self, domain: &'static str, secure: bool) -> Cookie<'static> {
		let json = serde_json::to_string(self).expect("this is valid json");

		Cookie::build((Self::COOKIE_NAME, json))
			.domain(domain)
			.path("/")
			.secure(secure)
			.http_only(false)
			.permanent()
			.build()
	}
}

impl<'de> Deserialize<'de> for Player {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct Helper1 {
			response: Helper2,
		}

		#[derive(Deserialize)]
		struct Helper2 {
			players: [Helper3; 1],
		}

		#[derive(Deserialize)]
		struct Helper3 {
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
				steam_id64: player.steamid.as_u64().to_string(),
				username: player.personaname,
				realname: player.realname,
				country: player.loccountrycode,
				profile_url: player.profileurl,
				avatar_url: player.avatar,
			},
		)
	}
}
