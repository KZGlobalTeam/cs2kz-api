use color_eyre::eyre::ensure;
use color_eyre::Result;
use cs2kz::SteamID;
use reqwest::{header, StatusCode};
use serde_json::json;
use tracing::info;

use super::Context;
use crate::auth::servers::routes::refresh_key::ServerAuthRequest;
use crate::auth::servers::ServerAccessToken;
use crate::players::Player;

#[crate::test]
async fn register_player(ctx: Context) {
	let access_token = get_access_token(&ctx).await?;
	let steam_id = "STEAM_1:0:448781326".parse::<SteamID>()?;
	let player = json!({
	  "steam_id": steam_id,
	  "name": "Szwagi",
	  "ip_address": "127.0.0.1"
	});

	let url = ctx.url("/players");
	let response = ctx
		.http_client
		.post(url)
		.header(header::AUTHORIZATION, format!("Bearer {access_token}"))
		.json(&player)
		.send()
		.await?;

	ensure!(response.status() == StatusCode::CREATED, "got {}", response.status());

	let url = ctx.url("/players/szwagi");
	let szwagi = ctx
		.http_client
		.get(url)
		.send()
		.await?
		.json::<Player>()
		.await?;

	ensure!(szwagi.steam_id == steam_id);
	ensure!(szwagi.name == "Szwagi");
}

async fn get_access_token(ctx: &Context) -> Result<String> {
	let server = sqlx::query!("SELECT * FROM Servers LIMIT 1")
		.fetch_one(&ctx.connection_pool)
		.await?;

	let url = ctx.url("/auth/servers/refresh");
	let request_body = ServerAuthRequest {
		refresh_token: server.api_key.unwrap(),
		plugin_version: "0.0.1".parse()?,
	};

	let response = ctx.http_client.post(url).json(&request_body).send().await?;

	ensure!(response.status() == StatusCode::CREATED);

	let ServerAccessToken { access_token } = response.json().await?;

	info!("received token: `{access_token}`");

	Ok(access_token)
}
