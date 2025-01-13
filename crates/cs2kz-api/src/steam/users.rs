use steam_id::SteamId;
use url::Url;

use crate::steam;

/// Steam Web API URL for fetching user information.
const USER_URL: &str = "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002";

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SteamUser {
    #[schema(value_type = crate::openapi::shims::SteamId)]
    pub id: SteamId,
    pub name: String,

    #[debug("{:?}", profile_url.as_str())]
    pub profile_url: Url,

    #[debug("{:?}", avatar_url.as_str())]
    pub avatar_url: Url,
}

#[tracing::instrument(skip(http_client), ret(level = "debug"), err(level = "debug"))]
pub async fn fetch_user(
    http_client: &reqwest::Client,
    web_api_key: &str,
    steam_id: SteamId,
) -> Result<Option<SteamUser>, steam::ApiError> {
    #[derive(serde::Serialize)]
    struct Query<'a> {
        #[serde(rename = "key")]
        api_key: &'a str,

        #[serde(rename = "steamids", serialize_with = "SteamId::serialize_u64")]
        steam_id: SteamId,
    }

    steam::request(
        http_client
            .get(USER_URL)
            .query(&Query { api_key: web_api_key, steam_id }),
    )
    .await
    .map(|FetchPlayerResponse { mut players }| {
        let player = if players.is_empty() {
            return None;
        } else {
            players.remove(0)
        };

        Some(SteamUser {
            id: player.steamid,
            name: player.personaname,
            profile_url: player.profileurl,
            avatar_url: player.avatarmedium,
        })
    })
}

#[derive(Debug, serde::Deserialize)]
struct FetchPlayerResponse {
    players: Vec<PlayerObject>,
}

#[derive(Debug, serde::Deserialize)]
struct PlayerObject {
    steamid: SteamId,
    personaname: String,

    #[debug("{:?}", profileurl.as_str())]
    profileurl: Url,

    #[debug("{:?}", avatarmedium.as_str())]
    avatarmedium: Url,
}
