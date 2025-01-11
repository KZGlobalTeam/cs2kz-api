#[cfg(not(feature = "fake"))]
compile_error!("you must run this binary with `--features fake`");

use std::env;
use std::num::NonZero;

use anyhow::Context as _;
use cs2kz::Context;
use cs2kz::config::{Config, DatabaseConfig};
use cs2kz::git::GitRevision;
use cs2kz::maps::CourseFilterId;
use cs2kz::pagination::Limit;
use cs2kz::players::{self, GetPlayersParams, NewPlayer};
use cs2kz::plugin::{self, NewPluginVersion, PluginVersionId};
use cs2kz::records::{self, NewRecord};
use cs2kz::servers::{self, GetServersParams, NewServer, ServerHost};
use cs2kz::users::{self, CreateUserError, NewUser, UserId};
use fake::faker::company::en::Buzzword;
use fake::faker::name::en::{FirstName, LastName};
use fake::rand::prelude::SliceRandom;
use fake::{Fake, Faker};
use steam_id::SteamId;
use tracing::info;
use tracing_subscriber::EnvFilter;
use url::Url;

use self::cli::Resource;

mod cli;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("generator=trace,warn"))
        .init();

    let args = cli::args();

    let cfg = env::var("DATABASE_URL")
        .context("missing `DATABASE_URL` environment variable")?
        .parse::<Url>()
        .context("`DATABASE_URL` is not a valid URL")
        .map(|url| Config {
            database: DatabaseConfig {
                url,
                min_connections: 1,
                max_connections: Some(NonZero::<u32>::MIN),
            },
        })?;

    let cx = Context::new(cfg).await?;

    match (args.clear, args.resource) {
        (true, Resource::Players) => players::clear(&cx)
            .await
            .context("failed to delete players"),
        (false, Resource::Players) => generate_players(&cx, args.count).await,
        (true, Resource::Servers) => servers::clear(&cx)
            .await
            .context("failed to delete servers"),
        (false, Resource::Servers) => generate_servers(&cx, args.count).await,
        (true, Resource::Records { filter_id }) => records::clear(&cx, filter_id)
            .await
            .context("failed to delete records"),
        (false, Resource::Records { filter_id }) => {
            generate_records(&cx, args.count, filter_id).await
        },
    }
}

async fn generate_players(cx: &Context, count: u64) -> anyhow::Result<()> {
    for (player_id, player) in (0..count)
        .map(|_| Faker.fake::<NewPlayer>())
        .map(|player| (player.id, player))
    {
        players::register(cx, player).await?;

        info!(id = %player_id, "registered player");
    }

    Ok(())
}

async fn generate_servers(cx: &Context, count: u64) -> anyhow::Result<()> {
    let alphakeks_id = UserId::new(SteamId::from_u64(76561198282622073_u64)?);

    match users::create(cx, NewUser { id: alphakeks_id, name: "AlphaKeks" }).await {
        Ok(_) | Err(CreateUserError::UserAlreadyExists) => {},
        Err(CreateUserError::Database(error)) if error.is_unique_violation_of("PRIMARY") => {},
        Err(CreateUserError::Database(error)) => {
            anyhow::bail!("failed to create AlphaKeks: {error}")
        },
    }

    for _ in 0..count {
        let server_name = format!(
            "{} {}'s {}",
            FirstName().fake::<String>(),
            LastName().fake::<String>(),
            Buzzword().fake::<&str>(),
        );
        let server_host = Faker.fake::<ServerHost>();
        let server = NewServer {
            name: &server_name,
            host: &server_host,
            port: Faker.fake(),
            owner_id: alphakeks_id,
        };

        let (server_id, _) = servers::approve(cx, server).await?;

        info!(id = %server_id, "approved server");
    }

    Ok(())
}

async fn generate_records(
    cx: &Context,
    count: u64,
    filter_id: CourseFilterId,
) -> anyhow::Result<()> {
    let plugin_version_id = get_or_create_plugin_version(cx).await?;

    let player_ids = players::get(cx, GetPlayersParams { limit: Limit::MAX, ..Default::default() })
        .await?
        .map(|player| player.id)
        .collect::<Vec<_>>()
        .await?
        .into_inner();

    let server_ids = servers::get(cx, GetServersParams { limit: Limit::MAX, ..Default::default() })
        .await?
        .map(|server| server.id)
        .collect::<Vec<_>>()
        .await?
        .into_inner();

    for _ in 0..count {
        let record = NewRecord {
            filter_id,
            player_id: player_ids
                .choose(&mut fake::rand::thread_rng())
                .copied()
                .context("no players found; create some")?,
            server_id: server_ids
                .choose(&mut fake::rand::thread_rng())
                .copied()
                .context("no servers found; create some")?,
            plugin_version_id,
            ..Faker.fake()
        };

        let submitted = records::submit(cx, record)
            .await
            .context("failed to submit record")?;

        info!(id = %submitted.record_id, "submitted record");
    }

    Ok(())
}

async fn get_or_create_plugin_version(cx: &Context) -> anyhow::Result<PluginVersionId> {
    if let Some(version) = plugin::get_latest_version(cx).await? {
        return Ok(version.id);
    }

    let version = Faker.fake::<semver::Version>();
    let git_revision = Faker.fake::<GitRevision>();

    plugin::publish_version(cx, NewPluginVersion {
        version: &version,
        git_revision: &git_revision,
    })
    .await
    .context("failed to create plugin version")
}
