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
pub async fn fetch_users(
    http_client: &reqwest::Client,
    web_api_key: &str,
    steam_ids: &[SteamId],
) -> Result<Vec<SteamUser>, steam::ApiError> {
    #[derive(serde::Serialize)]
    struct Query<'a> {
        #[serde(rename = "key")]
        api_key: &'a str,

        #[serde(rename = "steamids")]
        steam_ids: &'a str,
    }

    let ids = steam_ids
        .iter()
        .map(|id| id.as_u64().to_string())
        .collect::<Vec<_>>()
        .join(",");

    steam::request(
        http_client
            .get(USER_URL)
            .query(&Query { api_key: web_api_key, steam_ids: &ids }),
    )
    .await
    .map(|FetchPlayerResponse { players }| {
        players
            .into_iter()
            .map(|player| SteamUser {
                id: player.steamid,
                name: player.personaname,
                profile_url: player.profileurl,
                avatar_url: player.avatarmedium,
            })
            .collect()
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
