use color_eyre::Result;
use cs2kz::SteamID;
use reqwest::{header, StatusCode};
use serde_json::json;
use tracing::info;

use super::Context;
use crate::models::Player;
use crate::routes::auth::{AuthRequest, AuthResponse};

struct Server {
	token: String,
}

impl Server {
	async fn get(ctx: &Context) -> Result<Self> {
		let server = sqlx::query!("SELECT * FROM Servers LIMIT 1")
			.fetch_one(&ctx.pool)
			.await?;

		let url = ctx.url("/auth/refresh");
		let request_body =
			AuthRequest { api_key: server.api_key.unwrap(), plugin_version: "0.0.1".parse()? };

		let response = ctx.client.post(url).json(&request_body).send().await?;

		assert_eq!(response.status(), StatusCode::CREATED);

		let AuthResponse { token } = response.json().await?;

		info!("received token: `{token}`");

		Ok(Self { token })
	}
}

#[crate::test]
async fn register_player(ctx: Context) {
	let server = Server::get(&ctx).await?;
	let steam_id = "STEAM_1:0:448781326".parse::<SteamID>()?;
	let player = json!({
	  "steam_id": steam_id,
	  "name": "Szwagi",
	  "ip_address": "127.0.0.1"
	});

	let url = ctx.url("/players");
	let response = ctx
		.client
		.post(url)
		.header(header::AUTHORIZATION, format!("Bearer {}", server.token))
		.json(&player)
		.send()
		.await?;

	assert_eq!(response.status(), StatusCode::CREATED);

	let url = ctx.url("/players/szwagi");
	let szwagi = ctx.client.get(url).send().await?.json::<Player>().await?;

	assert_eq!(szwagi.steam_id, steam_id);
	assert_eq!(szwagi.name, "Szwagi");
}
